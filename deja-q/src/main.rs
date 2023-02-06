/*
 * G--
 * -PG
 * ---
 */

use std::{thread::sleep, time::Duration};

use rand::{prelude::SliceRandom, Rng};

#[derive(Copy, Clone)]
enum Tile {
    Pacman,
    Ghost,
    Empty,
    Wall,
}

fn clear_screen() {
    print!("{esc}c", esc = 27 as char);
}

struct Board {
    tiles: Vec<Vec<Tile>>,
}

impl Board {
    fn new(width: usize, height: usize) -> Self {
        Board {
            tiles: vec![vec![Tile::Empty; width]; height],
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
        for row in self.tiles.iter() {
            for tile in row {
                print!(
                    "{}",
                    match tile {
                        Tile::Pacman => '🤪',
                        Tile::Ghost => '👻',
                        Tile::Empty => '　',
                        Tile::Wall => '⬜',
                    }
                );
            }
            println!();
        }
    }

    fn tick(&mut self) {
        let ghost_points = self
            .tiles
            .iter()
            .enumerate()
            .flat_map(|(y, row)| {
                row.iter()
                    .enumerate()
                    .filter_map(move |(x, tile)| match tile {
                        Tile::Ghost => Some((x, y)),
                        _ => None,
                    })
            })
            .collect::<Vec<_>>();
        let mut rng = rand::thread_rng();
        for (x, y) in ghost_points.iter() {
            let available_moves = [(*x, *y - 1), (*x, *y + 1), (*x - 1, *y), (*x + 1, *y)]
                .into_iter()
                .filter(|(x, y)| matches!(self.tiles[*y][*x], Tile::Empty | Tile::Pacman))
                .collect::<Vec<_>>();

            if let Some((xp, yp)) = available_moves.choose(&mut rng) {
                self.tiles[*y][*x] = Tile::Empty;
                self.tiles[*yp][*xp] = Tile::Ghost;
            }
        }
    }
}

fn main() {
    let mut board = Board::empty_with_edge_walls(10, 10);
    for x in 1..5 {
        board.tiles[x][1] = Tile::Ghost;
        board.tiles[x][2] = Tile::Ghost;
        board.tiles[x][3] = Tile::Ghost;
    }
    board.tiles[5][5] = Tile::Pacman;
    loop {
        board.render();
        sleep(Duration::from_millis(100));
        board.tick();
    }
}
