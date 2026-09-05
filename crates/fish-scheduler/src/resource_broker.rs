use std::collections::HashMap;
use std::sync::Arc;

use fish_executor::ResourceRequirements;
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};

pub struct HostResourceGuard {
    _host_permit: OwnedSemaphorePermit,
    _token_permits: Vec<OwnedSemaphorePermit>,
}

#[derive(Clone)]
pub struct HostResourceBroker {
    total_permits: usize,
    host_semaphore: Arc<Semaphore>,
    token_semaphores: Arc<Mutex<HashMap<String, Arc<Semaphore>>>>,
}

impl HostResourceBroker {
    pub fn new(total_permits: usize) -> Self {
        let permits = total_permits.max(1);
        Self {
            total_permits: permits,
            host_semaphore: Arc::new(Semaphore::new(permits)),
            token_semaphores: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn total_permits(&self) -> usize {
        self.total_permits
    }

    pub fn available_permits(&self) -> usize {
        self.host_semaphore.available_permits()
    }

    async fn get_token_semaphore(&self, token: &str) -> Arc<Semaphore> {
        let mut map = self.token_semaphores.lock().await;
        map.entry(token.to_string())
            .or_insert_with(|| Arc::new(Semaphore::new(1)))
            .clone()
    }

    pub async fn acquire(&self, req: &ResourceRequirements) -> HostResourceGuard {
        let mut sorted_tokens = req.tokens.clone();
        sorted_tokens.sort();
        sorted_tokens.dedup();

        let mut token_permits = Vec::with_capacity(sorted_tokens.len());
        for token in sorted_tokens {
            let sem = self.get_token_semaphore(&token).await;
            let permit = sem.acquire_owned().await.expect("semaphore is not closed");
            token_permits.push(permit);
        }

        let needed_permits = if req.exclusive {
            self.total_permits as u32
        } else {
            req.permits.clamp(1, self.total_permits) as u32
        };

        let host_permit = self
            .host_semaphore
            .clone()
            .acquire_many_owned(needed_permits)
            .await
            .expect("host semaphore is not closed");

        HostResourceGuard {
            _host_permit: host_permit,
            _token_permits: token_permits,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_broker_permit_acquisition() {
        let broker = HostResourceBroker::new(4);
        assert_eq!(broker.available_permits(), 4);

        let req = ResourceRequirements {
            permits: 2,
            tokens: Vec::new(),
            exclusive: false,
        };

        let guard = broker.acquire(&req).await;
        assert_eq!(broker.available_permits(), 2);

        drop(guard);
        assert_eq!(broker.available_permits(), 4);
    }

    #[tokio::test]
    async fn test_broker_exclusive_access() {
        let broker = HostResourceBroker::new(4);

        let req_exclusive = ResourceRequirements {
            permits: 1,
            tokens: Vec::new(),
            exclusive: true,
        };

        let guard = broker.acquire(&req_exclusive).await;
        assert_eq!(broker.available_permits(), 0);

        drop(guard);
        assert_eq!(broker.available_permits(), 4);
    }

    #[tokio::test]
    async fn test_token_mutual_exclusion() {
        let broker = Arc::new(HostResourceBroker::new(4));
        let token = "exclusive_linker";

        let broker1 = broker.clone();
        let broker2 = broker.clone();

        let req = ResourceRequirements {
            permits: 1,
            tokens: vec![token.to_string()],
            exclusive: false,
        };

        let task1 = tokio::spawn(async move {
            let _guard = broker1.acquire(&req).await;
            sleep(Duration::from_millis(50)).await;
            1
        });

        let task2 = tokio::spawn(async move {
            sleep(Duration::from_millis(10)).await;
            let req2 = ResourceRequirements {
                permits: 1,
                tokens: vec![token.to_string()],
                exclusive: false,
            };
            let _guard = broker2.acquire(&req2).await;
            2
        });

        let (r1, r2) = tokio::join!(task1, task2);
        assert_eq!(r1.unwrap(), 1);
        assert_eq!(r2.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_deadlock_free_token_sorting() {
        let broker = Arc::new(HostResourceBroker::new(4));

        let b1 = broker.clone();
        let b2 = broker.clone();

        let req1 = ResourceRequirements {
            permits: 1,
            tokens: vec!["token_B".to_string(), "token_A".to_string()],
            exclusive: false,
        };

        let req2 = ResourceRequirements {
            permits: 1,
            tokens: vec!["token_A".to_string(), "token_B".to_string()],
            exclusive: false,
        };

        let t1 = tokio::spawn(async move {
            let _g = b1.acquire(&req1).await;
            sleep(Duration::from_millis(20)).await;
        });

        let t2 = tokio::spawn(async move {
            let _g = b2.acquire(&req2).await;
            sleep(Duration::from_millis(20)).await;
        });

        tokio::try_join!(t1, t2).unwrap();
    }
}
