//! Authentication descriptors.

/// Authentication strategy. Phase 0: basic only.
#[derive(Clone, Debug)]
pub enum Auth {
    Basic { user: String, password: String },
}

pub fn basic(user: impl Into<String>, password: impl Into<String>) -> Auth {
    Auth::Basic { user: user.into(), password: password.into() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn basic_constructs() {
        let a = basic("neo4j", "test");
        match a { Auth::Basic { user, password } => {
            assert_eq!(user, "neo4j");
            assert_eq!(password, "test");
        } }
    }
}
