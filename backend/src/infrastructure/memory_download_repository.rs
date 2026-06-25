use crate::domain::models::ActiveDownload;
use crate::domain::ports::download_repository::DownloadRepository;
use std::sync::RwLock;
use std::collections::HashMap;

pub struct InMemoryDownloadRepository {
    downloads: RwLock<HashMap<String, ActiveDownload>>,
    handles: RwLock<HashMap<String, tokio::task::JoinHandle<()>>>,
}

impl InMemoryDownloadRepository {
    pub fn new() -> Self {
        Self {
            downloads: RwLock::new(HashMap::new()),
            handles: RwLock::new(HashMap::new()),
        }
    }
}

impl DownloadRepository for InMemoryDownloadRepository {
    fn add_download(&self, download: ActiveDownload) {
        let mut map = self.downloads.write().unwrap();
        map.insert(download.id.clone(), download);
    }

    fn update_download(&self, id: &str, update_fn: Box<dyn FnOnce(&mut ActiveDownload) + Send>) {
        let mut map = self.downloads.write().unwrap();
        if let Some(download) = map.get_mut(id) {
            update_fn(download);
        }
    }

    fn remove_download(&self, id: &str) -> Option<ActiveDownload> {
        // Abort the handle if it is active
        {
            let mut handles_map = self.handles.write().unwrap();
            if let Some(handle) = handles_map.remove(id) {
                tracing::info!("Aborting active download task: {}", id);
                handle.abort();
            }
        }
        let mut map = self.downloads.write().unwrap();
        map.remove(id)
    }

    fn list_downloads(&self) -> Vec<ActiveDownload> {
        let map = self.downloads.read().unwrap();
        map.values().cloned().collect()
    }

    fn register_handle(&self, id: &str, handle: tokio::task::JoinHandle<()>) {
        let mut map = self.handles.write().unwrap();
        map.insert(id.to_string(), handle);
    }

    fn deregister_handle(&self, id: &str) {
        let mut map = self.handles.write().unwrap();
        map.remove(id);
    }
}
