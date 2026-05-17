use rand::Rng;

const ADJECTIVES: &[&str] = &[
    "Sneaky",
    "Wobbly",
    "Dancing",
    "Sleepy",
    "Curious",
    "Grumpy",
    "Cheerful",
    "Mysterious",
    "Fluffy",
    "Sparkling",
    "Whispering",
    "Brave",
    "Shy",
    "Bold",
    "Quirky",
    "Silly",
    "Mighty",
    "Tiny",
    "Cosmic",
    "Frosty",
    "Spicy",
    "Loud",
    "Polite",
    "Rebel",
    "Lucky",
    "Restless",
    "Tipsy",
    "Drowsy",
    "Witty",
    "Jolly",
    "Plucky",
    "Smug",
    "Zesty",
    "Bouncy",
    "Velvet",
    "Salty",
    "Glittery",
    "Turbo",
    "Stealthy",
    "Galactic",
    "Feral",
    "Nimble",
    "Caffeinated",
    "Honest",
    "Posh",
    "Royal",
    "Humble",
    "Suspicious",
    "Charming",
    "Dapper",
];

const ANIMALS: &[&str] = &[
    "Penguin",
    "Walrus",
    "Octopus",
    "Llama",
    "Otter",
    "Hedgehog",
    "Capybara",
    "Narwhal",
    "Platypus",
    "Quokka",
    "Axolotl",
    "Pangolin",
    "Sloth",
    "Lemur",
    "Manatee",
    "Wombat",
    "Raccoon",
    "Badger",
    "Mongoose",
    "Ferret",
    "Flamingo",
    "Puffin",
    "Toucan",
    "Kiwi",
    "Ocelot",
    "Tapir",
    "Aardvark",
    "Armadillo",
    "Jellyfish",
    "Stingray",
    "Cuttlefish",
    "Seahorse",
    "Beluga",
    "Dingo",
    "Meerkat",
    "Marmoset",
    "Tarsier",
    "Chinchilla",
    "Alpaca",
    "Yak",
    "Kangaroo",
    "Wallaby",
    "Koala",
    "Possum",
    "Iguana",
    "Chameleon",
    "Gecko",
    "Salamander",
    "Newt",
    "Crab",
];

pub fn generate_funny_name() -> String {
    let mut rng = rand::rng();
    let adj = ADJECTIVES[rng.random_range(0..ADJECTIVES.len())];
    let animal = ANIMALS[rng.random_range(0..ANIMALS.len())];
    format!("{adj} {animal}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn generate_funny_name_returns_adjective_animal_pair() {
        let name = generate_funny_name();
        let parts: Vec<&str> = name.split(' ').collect();
        assert_eq!(parts.len(), 2, "expected 'Adjective Animal', got {name:?}");
        assert!(
            ADJECTIVES.contains(&parts[0]),
            "unknown adjective: {}",
            parts[0]
        );
        assert!(ANIMALS.contains(&parts[1]), "unknown animal: {}", parts[1]);
    }

    #[test]
    fn generate_funny_name_samples_are_not_all_identical() {
        // With ~2500 combos, 40 identical draws would be astronomically unlikely.
        let names: HashSet<String> = (0..40).map(|_| generate_funny_name()).collect();
        assert!(names.len() > 1);
    }
}
