//! `Context` is a top level module contains static context and dynamic context for each request
use std::sync::Arc;

use diesel::connection::AnsiTransactionManager;
use diesel::pg::Pg;
use diesel::Connection;
use futures_cpupool::CpuPool;
use r2d2::{ManageConnection, Pool};

use stq_http::client::ClientHandle;
use stq_router::RouteParser;
use stq_types::UserId;

use super::routes::*;
use config::Config;
use repos::repo_factory::*;
use services::emarsys::EmarsysClient;
use services::sendgrid::SendgridService;

/// Static context for all app
pub struct StaticContext<T, M, F>
where
    T: Connection<Backend = Pg, TransactionManager = AnsiTransactionManager> + 'static,
    M: ManageConnection<Connection = T>,
    F: ReposFactory<T>,
{
    pub db_pool: Pool<M>,
    pub cpu_pool: CpuPool,
    pub config: Arc<Config>,
    pub route_parser: Arc<RouteParser<Route>>,
    pub client_handle: ClientHandle,
    pub repo_factory: F,
    pub emarsys_client: Arc<EmarsysClient>,
    pub sendgrid_service: Arc<SendgridService>,
}

impl<
        T: Connection<Backend = Pg, TransactionManager = AnsiTransactionManager> + 'static,
        M: ManageConnection<Connection = T>,
        F: ReposFactory<T>,
    > StaticContext<T, M, F>
{
    /// Create a new static context
    pub fn new(
        db_pool: Pool<M>,
        cpu_pool: CpuPool,
        client_handle: ClientHandle,
        config: Arc<Config>,
        repo_factory: F,
        emarsys_client: Arc<EmarsysClient>,
        sendgrid_service: Arc<SendgridService>,
    ) -> Self {
        let route_parser = Arc::new(create_route_parser());
        Self {
            route_parser,
            db_pool,
            cpu_pool,
            client_handle,
            config,
            repo_factory,
            emarsys_client,
            sendgrid_service,
        }
    }
}

impl<
        T: Connection<Backend = Pg, TransactionManager = AnsiTransactionManager> + 'static,
        M: ManageConnection<Connection = T>,
        F: ReposFactory<T>,
    > Clone for StaticContext<T, M, F>
{
    fn clone(&self) -> Self {
        Self {
            cpu_pool: self.cpu_pool.clone(),
            db_pool: self.db_pool.clone(),
            route_parser: self.route_parser.clone(),
            client_handle: self.client_handle.clone(),
            config: self.config.clone(),
            repo_factory: self.repo_factory.clone(),
            emarsys_client: self.emarsys_client.clone(),
            sendgrid_service: self.sendgrid_service.clone(),
        }
    }
}

#[derive(Clone)]
pub struct DynamicContext {
    pub user_id: Option<UserId>,
    pub correlation_token: String,
}

impl DynamicContext {
    /// Create a new dynamic context for each request
    pub fn new(user_id: Option<UserId>, correlation_token: String) -> Self {
        Self {
            user_id,
            correlation_token,
        }
    }
}
