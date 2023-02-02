#![cfg(test)]

use rand::{distributions::Uniform, Rng};

/// Random 12-character uppercase text.
pub fn random_upper() -> String {
    let mut rng = rand::thread_rng();
    let upper = Uniform::from(b'A'..=b'Z');
    (0..11).map(|_| rng.sample(upper) as char).collect()
}

#[cfg(test)]
mod tests {
    use super::random_upper;

    #[test]
    fn when_generating_random_text_then_the_text_is_unique() {
        // Arrange/Act
        let text_1 = random_upper();
        let text_2 = random_upper();

        // Assert
        assert_ne!(text_1, text_2);
    }
}
