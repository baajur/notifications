UPDATE templates 
SET data = '<html>
  <head>
    <title>Successful registration on Storiqa</title>
  </head>
  <body>
    <p>
      Dear {{user.first_name}},
      <br/>
      Your e-mail address is successfully confirmed and registration process is completely finished. Thank you for joining us!
      <br/>
      Best regards,
      Storiqa Team  
      <br/>
      <i>This is an automatically generated e-mail – please do not reply to it.</i>

    </p>

  </body>
</html>'
WHERE
name = 'apply_email_verification_for_user';