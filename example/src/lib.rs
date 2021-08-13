pub struct Example;

#[ezrpc::tower]
impl Example {
    pub async fn reverse(string: String) -> Result<String, EmptyString> {
        if !string.is_empty() {
            Ok(string.chars().rev().collect())
        } else {
            Err(EmptyString)
        }
    }

    pub async fn echo(string: String) -> String {
        string
    }
}

#[derive(Debug)]
pub struct EmptyString;
