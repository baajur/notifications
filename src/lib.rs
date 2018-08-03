extern crate base64;
extern crate chrono;
extern crate config as config_crate;
extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate futures_cpupool;
extern crate hyper;
extern crate hyper_tls;
extern crate jsonwebtoken;
#[macro_use]
extern crate log;
extern crate rand;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate handlebars;
extern crate lettre;
extern crate lettre_email;
extern crate mime;
extern crate native_tls;
extern crate notify;
extern crate serde_json;
extern crate sha3;
extern crate tokio_core;
extern crate uuid;

extern crate stq_http;
extern crate stq_logging;
extern crate stq_router;
extern crate stq_static_resources;

pub mod config;
pub mod controller;
pub mod errors;
pub mod models;
pub mod services;

use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::process;
use std::sync::{mpsc::channel, Arc, Mutex};
use std::thread;
use std::time::Duration;

use futures::future;
use futures::prelude::*;
use futures_cpupool::CpuPool;
use hyper::server::Http;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use tokio_core::reactor::Core;

use stq_http::controller::Application;

/// Starts new web service from provided `Config`
pub fn start_server(config: config::Config) {
    let thread_count = config.server.thread_count;
    let cpu_pool = CpuPool::new(thread_count);
    // Prepare reactor
    let mut core = Core::new().expect("Unexpected error creating event loop core");
    let handle = Arc::new(core.handle());

    let address = config.server.address.parse().expect("Address must be set in configuration");

    let http_config = stq_http::client::Config {
        http_client_retries: config.client.http_client_retries,
        http_client_buffer_size: config.client.http_client_buffer_size,
    };
    let client = stq_http::client::Client::new(&http_config, &handle);
    let client_handle = client.handle();
    let client_stream = client.stream();
    handle.spawn(client_stream.for_each(|_| Ok(())));

    let template_dir = config
        .templates
        .clone()
        .map(|t| t.path)
        .unwrap_or_else(|| format!("{}/templates", env!("OUT_DIR")));

    let templates = Arc::new(Mutex::new(HashMap::new()));

    for entry in fs::read_dir(template_dir.clone()).unwrap() {
        let entry = entry.unwrap();
        if !entry.file_type().unwrap().is_dir() {
            let path = entry.path();
            let p = path.clone();
            let file_name = p.file_name().unwrap().to_str().unwrap();
            let mut file = File::open(path).unwrap();
            let mut template = String::new();
            file.read_to_string(&mut template).unwrap();
            let mut t = templates.lock().unwrap();
            t.insert(file_name.to_string(), template);
        }
    }

    let (tx, rx) = channel();

    let mut watcher = watcher(tx, Duration::from_secs(10)).unwrap();

    watcher.watch(template_dir, RecursiveMode::Recursive).unwrap();

    thread::spawn({
        let templates = templates.clone();
        move || loop {
            match rx.recv() {
                Ok(event) => match event {
                    DebouncedEvent::Write(p) => {
                        if let Some(file_name) = p.clone().file_name() {
                            if let Some(file_name) = file_name.to_str() {
                                let res = File::open(p).and_then(|mut file| {
                                    let mut template = String::new();
                                    file.read_to_string(&mut template).map(|_| {
                                        let mut t = templates.lock().unwrap();
                                        t.insert(file_name.to_string(), template);
                                    })
                                });
                                match res {
                                    Ok(_) => info!("Template {} updated successfully.", file_name),
                                    Err(e) => error!("Template {} updated with error - {}.", file_name, e),
                                }
                            }
                        }
                    }
                    _ => (),
                },
                Err(e) => error!("watch templates error: {:?}", e),
            }
        }
    });
    let controller = controller::ControllerImpl::new(config.clone(), cpu_pool.clone(), client_handle.clone(), templates.clone());
    let serve = Http::new()
        .serve_addr_handle(&address, &*handle, {
            move || {
                // Prepare application
                let app = Application::<errors::Error>::new(controller.clone());

                Ok(app)
            }
        })
        .unwrap_or_else(|reason| {
            eprintln!("Http Server Initialization Error: {}", reason);
            process::exit(1);
        });

    handle.spawn(
        serve
            .for_each({
                let handle = handle.clone();
                move |conn| {
                    handle.spawn(conn.map(|_| ()).map_err(|why| eprintln!("Server Error: {:?}", why)));
                    Ok(())
                }
            })
            .map_err(|_| ()),
    );

    info!("Listening on http://{}, threads: {}", address, thread_count);
    core.run(future::empty::<(), ()>()).unwrap();
}
