pub struct Aggregator {}

impl Aggregator {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn work(self) -> Summary {
        return Summary {};
    }
}

pub struct Summary {}
