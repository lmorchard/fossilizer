// todo: move this to config / cli options?
pub static CLIENT_NAME: &str = "Fossilizer";
pub static CLIENT_WEBSITE: &str = "https://lmorchard.github.io/fossilizer/";
pub static OAUTH_SCOPES: &str = "read read:notifications read:statuses write follow push";
pub static REDIRECT_URI_OOB: &str = "urn:ietf:wg:oauth:2.0:oob";

pub mod fetcher;
pub mod importer;
pub mod instance;
