use rand::Rng;

const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const GROUP_LEN: usize = 6;
const GROUPS: usize = 3;

pub fn generate() -> String {
    let mut rng = rand::rng();
    let mut parts = Vec::with_capacity(GROUPS);

    for _ in 0..GROUPS {
        let group: String = (0..GROUP_LEN)
            .map(|_| {
                let idx = rng.random_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
        parts.push(group);
    }

    parts.join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_three_groups_of_six() {
        let pw = generate();
        let groups: Vec<&str> = pw.split('-').collect();
        assert_eq!(groups.len(), 3);
        for group in &groups {
            assert_eq!(group.len(), 6);
        }
    }

    #[test]
    fn total_length() {
        let pw = generate();
        assert_eq!(pw.len(), 20);
    }

    #[test]
    fn only_alphanumeric() {
        let pw = generate();
        assert!(pw.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
    }

    #[test]
    fn unique_outputs() {
        let a = generate();
        let b = generate();
        assert_ne!(a, b);
    }
}
