use std::sync::mpsc;
use std::thread;

use tokio::sync::oneshot;

use crate::server::ServerCore;

type Job = Box<dyn FnOnce(&mut ServerCore) + Send + 'static>;

pub struct ServerWorker {
    sender: mpsc::Sender<Job>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl ServerWorker {
    #[must_use]
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel::<Job>();
        let join_handle = thread::Builder::new()
            .name("mcshader-lsp-core".to_owned())
            .spawn(move || {
                let mut core = ServerCore::new();
                while let Ok(job) = receiver.recv() {
                    job(&mut core);
                }
            })
            .unwrap();

        Self {
            sender,
            join_handle: Some(join_handle),
        }
    }

    pub async fn request<T, F>(&self, operation: F) -> T
    where
        T: Send + 'static,
        F: FnOnce(&mut ServerCore) -> T + Send + 'static,
    {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.sender
            .send(Box::new(move |core| {
                let _ = reply_tx.send(operation(core));
            }))
            .expect("language server worker channel closed unexpectedly");

        reply_rx
            .await
            .expect("language server worker stopped before replying")
    }
}

impl Drop for ServerWorker {
    fn drop(&mut self) {
        let (replacement_tx, _replacement_rx) = mpsc::channel::<Job>();
        let sender = std::mem::replace(&mut self.sender, replacement_tx);
        drop(sender);

        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}
