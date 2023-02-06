use std::{thread::sleep, time::Duration};
use rand::prelude::SliceRandom;

#[derive(Copy, Clone, PartialEq)]
enum Tile {
    Empty,
    Wall,
}

#[derive(Copy, Clone, PartialEq)]
enum Direction {
    Noop,
    Up,
    Down,
    Left,
    Right,
}

fn clear_screen() {
    print!("{esc}c", esc = 27 as char);
}

struct Board {
    tiles: Vec<Vec<Tile>>,
    ghosts: Vec<(usize, usize)>,
    pacman: (usize, usize),
}

trait PacmanAi {
    fn select_move(&mut self, surroundings: [Tile; 8]) -> Direction;
}

impl Board {
    fn new(width: usize, height: usize) -> Self {
        Board {
            tiles: vec![vec![Tile::Empty; width]; height],
            ghosts: vec![],
            pacman: (5, 5),
        }
    }

    fn empty_with_edge_walls(width: usize, height: usize) -> Self {
        let mut board = Board::new(width, height);
        for x in 0..width {
            board.tiles[0][x] = Tile::Wall;
            board.tiles[height - 1][x] = Tile::Wall;
        }
        for y in 1..(height - 1) {
            board.tiles[y][0] = Tile::Wall;
            board.tiles[y][width - 1] = Tile::Wall;
        }
        board
    }

    fn render(&self) {
        clear_screen();
        for y in 0..self.tiles.len() {
            for x in 0..self.tiles[0].len() {
                print!("{}", match self.tiles[y][x] {
                    Tile::Empty => {
                        if self.pacman == (x, y) {
                            '🤪'
                        } else if self.ghosts.contains(&(x, y)) {
                            '👻'
                        } else {
                            '　'
                        }
                    },
                    Tile::Wall => '⬜',
                });
            }
            println!();
        }
    }

    fn has_ghost_at(&self, x: usize, y: usize) -> bool {
        self.ghosts.contains(&(x, y))
    }

    fn tick_ghosts(&mut self) {
        let mut rng = rand::thread_rng();
        for i in 0..self.ghosts.len() {
            let (x, y) = self.ghosts[i];
            let available_moves = [(x, y - 1), (x, y + 1), (x - 1, y), (x + 1, y)]
                .into_iter()
                .filter(|(x, y)| matches!(self.tiles[*y][*x], Tile::Empty) && !self.has_ghost_at(*x, *y))
                .collect::<Vec<_>>();
            if let Some((xp, yp)) = available_moves.choose(&mut rng) {
                self.ghosts[i] = (*xp, *yp);
            }
        }
    }

    fn is_game_over(&self) -> bool {
        self.ghosts.contains(&self.pacman)
    }
}

struct QLearningPacman {
}

impl QLearningPacman {
    fn new() -> Self {
        QLearningPacman {}
    }
}

impl PacmanAi for QLearningPacman {
    fn select_move(&mut self, surroundings: [Tile; 8]) -> Direction {
        todo!()
    }
}

fn main() {
    let mut board = Board::empty_with_edge_walls(10, 10);
    for x in 1..5 {
        board.ghosts.push((x, 1));
        board.ghosts.push((x, 2));
        board.ghosts.push((x, 3));
    }
    loop {
        board.render();
        sleep(Duration::from_millis(100));
        board.tick_ghosts();
        if board.is_game_over() {
            println!("Game over... LOSER");
            break;
        }
    }
}
