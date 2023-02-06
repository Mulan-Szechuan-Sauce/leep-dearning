use std::{thread::sleep, time::Duration};
use rand::prelude::{SliceRandom, ThreadRng};

#[derive(Copy, Clone, PartialEq)]
enum Tile {
    Empty,
    Wall,
}

fn clear_screen() {
    print!("{esc}c", esc = 27 as char);
}

struct Board {
    tiles: Vec<Vec<Tile>>,
    ghosts: Vec<(usize, usize)>,
    pacman: (usize, usize),
    rng: ThreadRng,
}

trait PacmanAi {
    /// Should only move at most once! Otherwise it is a BUUUUUUG
    fn tick_pacman(&mut self, board: &mut Board);
}

impl Board {
    fn new(width: usize, height: usize) -> Self {
        Board {
            tiles: vec![vec![Tile::Empty; width]; height],
            ghosts: vec![],
            pacman: (5, 5),
            rng: rand::thread_rng(),
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
        for i in 0..self.ghosts.len() {
            let (x, y) = self.ghosts[i];
            let available_moves = [(x, y - 1), (x, y + 1), (x - 1, y), (x + 1, y)]
                .into_iter()
                .filter(|(x, y)| matches!(self.tiles[*y][*x], Tile::Empty) && !self.has_ghost_at(*x, *y))
                .collect::<Vec<_>>();
            if let Some((xp, yp)) = available_moves.choose(&mut self.rng) {
                self.ghosts[i] = (*xp, *yp);
            }
        }
    }

    fn is_game_over(&self) -> bool {
        let (x, y) = self.pacman;
        self.tiles[y][x] == Tile::Wall || self.ghosts.contains(&self.pacman)
    }
}

struct PaQman {
}

impl PaQman {
    fn new() -> Self {
        PaQman {}
    }
}

impl PacmanAi for PaQman {
    fn tick_pacman(&mut self, board: &mut Board) {
        let (x, y) = board.pacman;
        let next = *[(x, y - 1), (x, y + 1), (x - 1, y), (x + 1, y)].choose(&mut board.rng).unwrap();
        board.pacman = next;
    }
}

fn main() {
    let mut ai = PaQman::new();
    let mut board = Board::empty_with_edge_walls(10, 10);
    for x in 1..5 {
        board.ghosts.push((x, 1));
        board.ghosts.push((x, 2));
        board.ghosts.push((x, 3));
    }
    loop {
        board.render();
        sleep(Duration::from_millis(100));
        ai.tick_pacman(&mut board);
        if board.is_game_over() {
            println!("Game over... LOSER");
            break;
        }
        board.tick_ghosts();
        if board.is_game_over() {
            println!("Game over... LOSER");
            break;
        }
    }
}
