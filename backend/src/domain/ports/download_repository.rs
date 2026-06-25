use crate::domain::models::ActiveDownload;

pub trait DownloadRepository: Send + Sync {
    fn add_download(&self, download: ActiveDownload);
    fn update_download(&self, id: &str, update_fn: Box<dyn FnOnce(&mut ActiveDownload) + Send>);
    fn remove_download(&self, id: &str) -> Option<ActiveDownload>;
    fn list_downloads(&self) -> Vec<ActiveDownload>;
    fn register_handle(&self, id: &str, handle: tokio::task::JoinHandle<()>);
    fn deregister_handle(&self, id: &str);
}
