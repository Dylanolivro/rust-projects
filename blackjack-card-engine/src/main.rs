use rand::seq::SliceRandom;
use std::fmt;
use thiserror::Error;

#[derive(Error, Debug)]
enum GameError {
    #[error("Deck is empty")]
    EmptyDeck,

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}
#[derive(Debug, Clone, Copy, PartialEq)]
enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

trait Card {
    fn value(&self) -> u8;
    fn display(&self) -> String;
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PokerCard {
    rank: Rank,
    suit: Suit,
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Rank::Two => "2",
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5",
            Rank::Six => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "10",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
            Rank::Ace => "A",
        };
        write!(f, "{}", s)
    }
}
impl fmt::Display for Suit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let symbol = match self {
            Suit::Hearts => "♥",
            Suit::Diamonds => "♦",
            Suit::Clubs => "♣",
            Suit::Spades => "♠",
        };
        write!(f, "{}", symbol)
    }
}
impl fmt::Display for PokerCard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} of {}", self.rank, self.suit)
    }
}

impl Card for PokerCard {
    fn value(&self) -> u8 {
        match self.rank {
            Rank::Two => 2,
            Rank::Three => 3,
            Rank::Four => 4,
            Rank::Five => 5,
            Rank::Six => 6,
            Rank::Seven => 7,
            Rank::Eight => 8,
            Rank::Nine => 9,
            Rank::Ten | Rank::Jack | Rank::Queen | Rank::King => 10,
            Rank::Ace => 11,
        }
    }
    fn display(&self) -> String {
        format!("{}", self)
    }
}

struct Deck<T> {
    cards: Vec<T>,
}

impl<T: Card + Clone + fmt::Debug> Deck<T> {
    fn shuffle(&mut self) {
        self.cards.shuffle(&mut rand::thread_rng());
    }

    fn deal(&mut self) -> Result<T, GameError> {
        self.cards.pop().ok_or(GameError::EmptyDeck)
    }
}

impl Deck<PokerCard> {
    fn new() -> Self {
        let mut cards = Vec::new();

        for suit in [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades] {
            for rank in [
                Rank::Two,
                Rank::Three,
                Rank::Four,
                Rank::Five,
                Rank::Six,
                Rank::Seven,
                Rank::Eight,
                Rank::Nine,
                Rank::Ten,
                Rank::Jack,
                Rank::Queen,
                Rank::King,
                Rank::Ace,
            ] {
                cards.push(PokerCard { rank, suit });
            }
        }

        Deck { cards }
    }
}

struct Hand<T> {
    cards: Vec<T>,
}
impl<T: Card> Hand<T> {
    fn new() -> Self {
        Hand { cards: Vec::new() }
    }

    fn add(&mut self, card: T) {
        self.cards.push(card);
    }

    fn score(&self) -> u8 {
        let mut total = 0;
        let mut aces = 0;

        for card in &self.cards {
            let value = card.value();
            total += value;
            if value == 11 {
                aces += 1;
            }
        }

        while total > 21 && aces > 0 {
            total -= 10;
            aces -= 1;
        }

        total
    }

    fn display(&self) -> String {
        self.cards
            .iter()
            .map(|c| c.display())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn run() -> Result<(), GameError> {
    let mut deck = Deck::new();
    deck.shuffle();

    let mut player = Hand::new();
    let mut dealer = Hand::new();

    // Deal initial cards
    player.add(deck.deal()?);
    dealer.add(deck.deal()?);
    player.add(deck.deal()?);
    dealer.add(deck.deal()?);

    println!(
        "Your hand: {} (score: {})",
        player.display(),
        player.score()
    );
    println!("Dealer shows: {}", dealer.cards[0].display());

    // Player turn
    loop {
        println!("Hit or stand? (h/s)");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        match input.trim() {
            "h" | "hit" => {
                let card = deck.deal()?;
                println!("You drew: {}", card.display());
                player.add(card);
                println!(
                    "Your hand: {} (score: {})",
                    player.display(),
                    player.score()
                );

                if player.score() > 21 {
                    println!("Bust! You lose.");
                    return Ok(());
                }
            }
            "s" | "stand" => break,
            _ => {
                return Err(GameError::InvalidInput(input.trim().to_string()));
            }
        }
    }

    // Dealer turn
    println!(
        "Dealer hand: {} (score: {})",
        dealer.display(),
        dealer.score()
    );
    while dealer.score() < 17 {
        let card = deck.deal()?;
        println!("Dealer drew: {}", card.display());
        dealer.add(card);
        println!(
            "Dealer hand: {} (score: {})",
            dealer.display(),
            dealer.score()
        );
    }

    if dealer.score() > 21 {
        println!("Dealer busts! You win.");
        return Ok(());
    }

    // Determine winner
    let player_score = player.score();
    let dealer_score = dealer.score();

    println!("Final - You: {}, Dealer: {}", player_score, dealer_score);

    if player_score > dealer_score {
        println!("You win!");
    } else if dealer_score > player_score {
        println!("Dealer wins!");
    } else {
        println!("Push! It's a tie.");
    }

    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
