#![cfg_attr(not(feature = "std"), no_std, no_main)]
extern crate alloc;

use pink_extension as pink;


#[pink::contract(env=PinkEnvironment)]
mod oh_my_chess {
    use super::{pink};
    use pink::PinkEnvironment;
    use scale::{Encode, Decode};
    use serde::{Deserialize, Serialize};
    use alloc::string::String;
    use alloc::format;
    use serde_json_core;
    use crate::oh_my_chess::Error::{NoElementFoundInDB, ErrorFetchingFromDB};
    use scale_info::TypeInfo;


    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        InvalidMove,
        NotYourTurn,
        NoElementFoundInDB,
        ErrorFetchingFromDB,
    }
    pub type Result<T> = core::result::Result<T, Error>;
    pub type Option<T> = core::option::Option<T>;


    #[ink(storage)]
    pub struct OhMyChess {
        admin: AccountId,
        url: String,
        api_key: String,
    }

    impl OhMyChess {

        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                url: String::from(""),
                api_key: String::from(""),
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

        // #[ink(message)]
        // pub fn make_move(&self, chess_move: ChessMove, player_to_move: Player) -> Result<bool, Error> {
        //     let game_state = self.find_game_session_from_mongodb()?;
        //
        //     // let is_valid_move = self.check_move_validity(&chess_move, &game_state);
        // }

        #[ink(message)]
        pub fn find_game_session_from_mongodb(&self, session_id: String) -> Result<GameState> {
            let method = String::from("POST"); // HTTP Method for the request
            let url = format!("{}/action/findOne?_id={}", self.url, session_id);

            let data = r#"{
                "collection":"game_sessions",
                "database":"hackathon",
                "dataSource":"Cluster0",
                "projection":{"_id":0,"turn":1,"status":1,"players":1,"board":1}
            }"#.as_bytes().to_vec();

            // Prepare headers
            let headers = alloc::vec![
                (String::from("Content-Type"), String::from("application/json")),
                (String::from("Access-Control-Request-Headers"), String::from("*")),
                (String::from("api-key"), self.api_key.clone()),
            ];

            let response = pink::http_req!(
                method,
                url,
                data,
                headers
            );

            let game_state_res = serde_json_core::from_slice::<MongoDBDocument>(response.body.as_slice())
                .map_err(|_| { ErrorFetchingFromDB })
                .map(|(mongodb, _)| { mongodb.document })?
                .ok_or(NoElementFoundInDB);

            game_state_res
        }

        pub fn check_move_validity(&self, chess_move: &ChessMove, player_to_move: &Player, game_state: &GameState) -> bool {
            // Check if the move is within the board boundaries
            if chess_move.from.0 > 7 || chess_move.from.1 > 7 || chess_move.to.0 > 7 || chess_move.to.1 > 7 {
                return false;
            }

            let ChessCell{ player, piece } = match game_state.board[chess_move.from.0 as usize][chess_move.from.1 as usize] {
                Some(ref chess_cell) => chess_cell,
                None => return false, // No piece at source
            };

            if player_to_move != player {
                return false;
            }

            let is_valid = match piece {
                Piece::Pawn => self.check_move_validity_pawn(player_to_move, chess_move, game_state),
                Piece::Knight => self.check_move_validity_knight(player_to_move, chess_move, game_state),
                Piece::Bishop => self.check_bishop_move_validity(chess_move, game_state),
                Piece::Rook => self.check_move_validity_rook(chess_move, game_state),
                Piece::Queen => self.check_move_validity_queen(chess_move, game_state),
                Piece::King => self.check_move_validity_king(chess_move),
            };

            is_valid
        }

        pub fn check_move_validity_pawn(&self, player: &Player, chess_move: &ChessMove, game_state: &GameState) -> bool {
            let (fx, fy) = chess_move.from;
            let (tx, ty) = chess_move.to;

            let forward = match player {
                Player::White => 1,
                Player::Black => -1,
            };

            // Check forward move
            if fx as i32 + forward == tx as i32 && fy == ty {
                return game_state.board[tx as usize][ty as usize].is_none();
            }

            // Check capture move
            if fx as i32 + forward == tx as i32 && (fy as i32 - 1 == ty as i32 || fy as i32 + 1 == ty as i32) {
                if let Some(ChessCell{player: piece_player, ..}) = &game_state.board[tx as usize][ty as usize] {
                    return *player != *piece_player; // Capture if it's an opponent's piece
                }
            }

            false
        }

        pub fn check_move_validity_knight(&self, player: &Player, chess_move: &ChessMove, game_state: &GameState) -> bool {
            let (fx, fy, tx, ty) = (chess_move.from.0, chess_move.from.1, chess_move.to.0, chess_move.to.1);
            let dx = (fx as i32 - tx as i32).abs();
            let dy = (fy as i32 - ty as i32).abs();

            // Check L-shape move
            if (dx == 2 && dy == 1) || (dx == 1 && dy == 2) {
                if let Some(ChessCell{player: piece_player, ..}) = &game_state.board[tx as usize][ty as usize] {
                    return *player != *piece_player; // Capture if there's an opponent piece
                } else { true }
            } else { false }

        }

        pub fn check_bishop_move_validity(&self, chess_move: &ChessMove, game_state: &GameState) -> bool {
            // Bishop can move diagonally
            let is_diagonal = (chess_move.from.0 as i32 - chess_move.to.0 as i32).abs() == (chess_move.from.1 as i32 - chess_move.to.1 as i32).abs();

            if is_diagonal {
                // Diagonal move: Ensure the path is clear
                self.is_path_clear(&(game_state.board), chess_move, &Direction::Diagonal)
            } else {
                false // Not a valid bishop move
            }
        }

        pub fn check_move_validity_rook(&self, chess_move: &ChessMove, game_state: &GameState) -> bool {
            // Rook can move horizontally or vertically
            let is_horizontal = chess_move.from.0 == chess_move.to.0;
            let is_vertical = chess_move.from.1 == chess_move.to.1;

            if is_horizontal {
                // Horizontal move: Ensure the path is clear
                self.is_path_clear(&(game_state.board), chess_move, &Direction::Horizontal)
            } else if is_vertical {
                // Vertical move: Ensure the path is clear
                self.is_path_clear(&(game_state.board), chess_move, &Direction::Vertical)
            } else {
                false // Not a valid rook move
            }
        }

        pub fn check_move_validity_king(&self, chess_move: &ChessMove) -> bool {
            // Calculate the difference in the move for both axes
            let delta_row = (chess_move.from.0 as i8 - chess_move.to.0 as i8).abs();
            let delta_col = (chess_move.from.1 as i8 - chess_move.to.1 as i8).abs();

            if delta_row <= 1 && delta_col <= 1 { true } else { false }
        }

        pub fn check_move_validity_queen(&self, chess_move: &ChessMove, game_state: &GameState) -> bool {
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

        fn is_path_clear(&self, board: &[[Option<ChessCell>; 8]; 8], chess_move: &ChessMove, direction: &Direction) -> bool {
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

    #[derive(Encode, Decode, Deserialize, Serialize, Clone, Debug, PartialEq, TypeInfo)]
    pub enum Direction {
        Horizontal,
        Vertical,
        Diagonal,
    }

    #[derive(Encode, Decode, Deserialize, Serialize, Clone, Debug, PartialEq, TypeInfo)]
    pub enum Piece {
        Pawn, Knight, Bishop, Rook, Queen, King
    }

    #[derive(Encode, Decode, Deserialize, Serialize, Clone, Debug, PartialEq, TypeInfo)]
    pub enum Player {
        Black, White
    }

    #[derive(Encode, Decode, Deserialize, Serialize, Clone, Debug, PartialEq, TypeInfo)]
    pub struct PlayersAddresses {
        black: [u8; 32],
        white: [u8; 32],
    }

    #[derive(Encode, Decode, Deserialize, Serialize, Clone, Debug, PartialEq, TypeInfo)]
    pub enum GameStatus {
        Ongoing, Checkmate, Stalemate, Draw
    }

    #[derive(Encode, Decode, Deserialize, Serialize, Clone, Debug, TypeInfo)]
    pub struct GameState {
        board: [[Option<ChessCell>; 8]; 8],
        turn: Player,
        players: PlayersAddresses,
        status: GameStatus,
    }

    #[derive(Encode, Decode, Deserialize, Serialize, Clone, Debug, PartialEq, TypeInfo)]
    pub struct ChessCell {
        piece: Piece,
        player: Player,
    }

    #[derive(Encode, Decode, Deserialize, Serialize, Clone, Debug, PartialEq, TypeInfo)]
    pub struct ChessMove {
        // Define your move structure
        from: (u8, u8),
        to: (u8, u8),
    }

    #[derive(Encode, Decode, Deserialize, Clone, Debug)]
    pub struct MongoDBDocument {
        // Define your move structure
        document: Option<GameState>
    }

    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope, so we can use them here.
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
