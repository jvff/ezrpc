pub struct Example;

#[ezrpc::tower]
impl Example {
    pub fn name() -> String {
        "Example".to_owned()
    }

    pub fn echo(&self, string: String) -> String {
        string
    }

    pub async fn reverse(&mut self, string: String) -> Result<String, EmptyString> {
        if !string.is_empty() {
            Ok(string.chars().rev().collect())
        } else {
            Err(EmptyString)
        }
    }
}

#[derive(Debug)]
pub struct EmptyString;
