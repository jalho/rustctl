pub struct Terminator {}

impl Terminator {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn work(self) -> Summary {
        return Summary {};
    }
}

pub struct Summary {}
