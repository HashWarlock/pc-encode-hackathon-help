#![cfg_attr(not(feature = "std"), no_std, no_main)]
extern crate alloc;

use pink_extension as pink;

#[pink::contract(env=PinkEnvironment)]
mod oh_my_chess {
    use super::{oh_my_chess, pink};
    use pink::PinkEnvironment;
    use scale::{Encode, Decode};
    use serde::Deserialize;
    use alloc::string::String;
    use alloc::format;
    // you have to use crates with `no_std` support in contract.
    use serde_json_core;
    // use mongodb::{Client, options::ClientOptions};

    // use alloc::collections::BTreeMap;
    // pub type GameId = u64;
    // Type alias for the contract's result type.
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        InvalidMove,
        NotYourTurn,
    }
    pub type Result<T> = core::result::Result<T, Error>;
    pub type Option<T> = core::option::Option<T>;


    #[ink(storage)]
    pub struct OhMyChess {
        admin: AccountId,
        url: String,
    }

    impl OhMyChess {

        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                url: String::default()
            }
        }

        #[ink(message)]
        pub fn get_url(&self) -> Result<String> {
            Ok(self.url.clone())
        }

        #[ink(message)]
        pub fn set_url(&mut self, url: String) {
            self.url = url.clone();
        }

        pub fn check_move_validity(&self, chess_move: &ChessMove, player_to_move: &Player, game_state: &GameState) -> bool {
            // Check if the move is within the board boundaries
            if chess_move.from.0 < 0 || chess_move.from.0 > 7 || chess_move.from.1 < 0 || chess_move.from.1 > 7 || chess_move.to.0 < 0 || chess_move.to.0 > 7 || chess_move.to.1 < 0 || chess_move.to.1 > 7 {
                return false;
            }

            let (piece, player) = match game_state.board[chess_move.from.0 as usize][chess_move.from.1 as usize] {
                Some(ref tuple) => tuple,
                None => return false, // No piece at source
            };

            if player_to_move != player {
                return false;
            }

            // let delta_row = (chess_move.from.0 as i8 - chess_move.to.0 as i8).abs();
            // let delta_col = (chess_move.from.1 as i8 - chess_move.to.1 as i8).abs();

            let is_valid = match piece {
                Piece::Pawn => true,
                Piece::Knight => true,
                Piece::Bishop => true,
                Piece::Rook => true,
                Piece::Queen => true,
                Piece::King => true,
            };

            is_valid
        }

        fn check_move_validity_king(&self, chess_move: &ChessMove) -> bool {
            // Calculate the difference in the move for both axes
            let delta_row = (chess_move.from.0 as i8 - chess_move.to.0 as i8).abs();
            let delta_col = (chess_move.from.1 as i8 - chess_move.to.1 as i8).abs();

            if delta_row <= 1 && delta_col <= 1 { true } else { false }
        }

        fn check_move_validity_queen(&self, chess_move: &ChessMove, game_state: &GameState) -> bool {
            // Queen can move horizontally, vertically, or diagonally
            let from = chess_move.from;
            let to = chess_move.to;
            let is_horizontal = from.0 == to.0;
            let is_vertical = from.1 == to.1;
            let is_diagonal = (from.0 as i32 - to.0 as i32).abs() == (from.1 as i32 - to.1 as i32).abs();

            if is_horizontal {
                self.is_path_clear(&(game_state.board), chess_move, &Direction::Horizontal)
            } else if is_vertical {
                self.is_path_clear(&(game_state.board), chess_move, &Direction::Vertical)
            } else if is_diagonal {
                self.is_path_clear(&(game_state.board), chess_move, &Direction::Diagonal)
            } else {
                false // Not a valid queen move
            }
        }

        fn is_path_clear(&self, board: &[[Option<(Piece, Player)>; 8]; 8], chess_move: &ChessMove, direction: &Direction) -> bool {
            let from = chess_move.from;
            let to = chess_move.to;
            let (dx, dy) = (to.0 as i32 - from.0 as i32, to.1 as i32 - from.1 as i32);
            let step_x = dx.signum() as u8;
            let step_y = dy.signum() as u8;

            // Check if movement is according to the piece's moving pattern
            match direction {
                Direction::Horizontal => if dy != 0 { return false; },
                Direction::Vertical => if dx != 0 { return false; },
                Direction::Diagonal => if dx.abs() != dy.abs() { return false; },
            }

            let mut current_x = from.0;
            let mut current_y = from.1;

            while (current_x, current_y) != (to.0, to.1) {
                current_x += step_x;
                current_y += step_y;

                // Avoid checking the destination square for a piece
                if (current_x, current_y) == (to.0, to.1) {
                    break;
                }

                // Check if the path is clear
                if board[current_x as usize][current_y as usize].is_some() {
                    return false;
                }
            }
            true
        }
    }

    #[derive(Encode, Decode, Clone, Debug, PartialEq)]
    pub enum Direction {
        Horizontal,
        Vertical,
        Diagonal,
    }

    #[derive(Encode, Decode, Clone, Debug, PartialEq)]
    pub enum Piece {
        Pawn, Knight, Bishop, Rook, Queen, King
    }

    #[derive(Encode, Decode, Clone, Debug, PartialEq)]
    pub enum Player {
        Black, White
    }

    #[derive(Encode, Decode, Clone, Debug, PartialEq)]
    pub enum GameStatus {
        Ongoing, Checkmate, Stalemate, Draw
    }

    #[derive(Encode, Decode, Clone, Debug)]
    pub struct GameState {
        board: [[Option<(Piece, Player)>; 8]; 8],
        turn: Player,
        players: (AccountId, AccountId),
        status: GameStatus,
    }

    #[derive(Encode, Decode, Clone, Debug, PartialEq)]
    pub struct ChessMove {
        // Define your move structure
        from: (u8, u8),
        to: (u8, u8),
    }

    #[cfg(test)]
    mod tests {
        use serde::__private::de::Content::String;
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// We test a simple use case of our contract.
        #[ink::test]
        fn it_works() {
            // when your contract is really deployed, the Phala Worker will do the HTTP requests
            // mock is needed for local test
            pink_extension_runtime::mock_ext::mock_all_ext();

            let oh_my_chess = OhMyChess::new();
            assert!(oh_my_chess.get_url(), "");

        }
    }

}
