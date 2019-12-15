use crate::askpass::UserInfo;
use crate::error::ErrorKind;
use pam::pam_sys::PamReturnCode;
use crate::askpass::simple::simple_get_credentials;
use pam::{Authenticator, PasswordConv, PamError};
use users::get_user_by_name;
use std::env;
use log::error;

pub fn initial_pam<'a>(tty: u32) -> Result<Authenticator<'a, PasswordConv>, PamError>{
    xdg(tty, 0, "greeter");

    let mut authenticator = Authenticator::with_password("login")
        .expect("Failed to init PAM client.");

    authenticator.open_session()?;

    Ok(authenticator)
}

fn xdg(tty: u32, _uid: u32, xdg_session_class: &str) {
    env::set_var("XDG_SESSION_CLASS", xdg_session_class);

    // seat0 is the "special" seat, meant for non-multiseat DMs / the first instance of a multiseat DM
    env::set_var("XDG_SEAT", "seat0");

    env::set_var("XDG_VTNR", format!("{}", tty));
    env::set_var("XDG_SESSION_ID", "1");

    if env::var("XDG_DATA_DIRS").is_err() {
        env::set_var("XDG_DATA_DIRS", "/usr/local/share/:/usr/share/")
    }

    env::set_var("XDG_SESSION_TYPE", "tty");
}

pub fn authenticate(authenticator: &mut Authenticator<PasswordConv>, tty: u32) -> Result<UserInfo, ErrorKind>{
//    let mut authenticator = Authenticator::with_password("login")
//        .expect("Failed to init PAM client.");

    // block where we inhibit suspend
    let login_info= {
        // TODO: change to generic get credentials
        let login_info = simple_get_credentials().map_err(|_| ErrorKind::IoError)?;

        let user= match get_user_by_name(&login_info.username){
            Some(i) => i,
            None => return Err(ErrorKind::AuthenticationError),
        };
        xdg(tty as u32, user.uid(), "user");

        authenticator.handler_mut().set_credentials(login_info.username.clone(), login_info.password);

        if let Err(e) = authenticator.authenticate() {
            if e.to_string() == PamReturnCode::PERM_DENIED.to_string() {
                error!("Permission denied.");
            } else if e.to_string() == PamReturnCode::AUTH_ERR.to_string() {
                #[cfg(debug_assertions)]
                error!("AUTH_ERR");

                println!("Authentication error.");
            } else if e.to_string() == PamReturnCode::USER_UNKNOWN.to_string() {
                #[cfg(debug_assertions)]
                error!("USER_UNKNOWN");

                println!("Authentication error.");
            } else if e.to_string() == PamReturnCode::MAXTRIES.to_string() {
                error!("Maximum login attempts reached.");
            } else if e.to_string() == PamReturnCode::CRED_UNAVAIL.to_string() {
                error!("Underlying authentication service can not retrieve user credentials unavailable.");
            } else if e.to_string() == PamReturnCode::ACCT_EXPIRED.to_string() {
                error!("Account expired");
            } else if e.to_string() == PamReturnCode::CRED_EXPIRED.to_string() {
                error!("Account  expired");
            } else if e.to_string() == PamReturnCode::TRY_AGAIN.to_string() {
                error!("PAM fucked up, please try again");
            } else if e.to_string() == PamReturnCode::ABORT.to_string() {
                error!("user's authentication token has expired");
            } else if e.to_string() == PamReturnCode::INCOMPLETE.to_string() {
                error!("We fucked up, please try again");
            } else {
                error!("A PAM error occurred: {}", e);
            }

            return Err(ErrorKind::AuthenticationError)
        };

        UserInfo{
            username: login_info.username,
            password: String::new()
        }
    };

    authenticator.open_session().map_err(|_| ErrorKind::SessionError)?;

    Ok(login_info)
}
