pub struct Example;

#[ezrpc::tower]
impl Example {
    pub fn echo(string: String) -> String {
        string
    }

    pub async fn reverse(string: String) -> Result<String, EmptyString> {
        if !string.is_empty() {
            Ok(string.chars().rev().collect())
        } else {
            Err(EmptyString)
        }
    }
}

#[derive(Debug)]
pub struct EmptyString;
