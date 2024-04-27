use core::ops::Deref;
use core::ops::DerefMut;
use image::GenericImageView;
use image::ImageBuffer;
use image::SubImage;
use image::{GenericImage, Pixel};

struct Grid {
    data: Vec<Vec<Seat>>,
    num_possibilities: usize,
    w: usize,
    h: usize,
}

impl Grid {
    fn render<'a, T: GenericImage<Pixel = P> + std::fmt::Debug, P: Pixel + std::cmp::PartialEq>(
        &self,
        tiles: Vec<SubImage<&T>>,
    ) -> ImageBuffer<P, Vec<<P as Pixel>::Subpixel>> {
        let tile_w = tiles[0].width();
        let tile_h = tiles[0].height();
        let mut output = image::ImageBuffer::new(self.w as u32 * tile_w, self.h as u32 * tile_h);
        for y in 0..self.h {
            for x in 0..self.w {
                let id = self[y][x].get_id().unwrap();
                let mut output_window =
                    output.sub_image(x as u32 * tile_w, y as u32 * tile_h, tile_w, tile_h);
                for (x, y, p) in tiles[id].pixels() {
                    output_window.put_pixel(x, y, p);
                }
            }
        }
        output
    }
}

impl Deref for Grid {
    type Target = Vec<Vec<Seat>>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for Grid {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Seat {
    Collapsed(usize),
    Uncertain(Vec<usize>),
}

impl Seat {
    fn get_id(&self) -> Result<usize, &str> {
        match *self {
            Seat::Collapsed(v) => Ok(v),
            Seat::Uncertain(_) => Err("Tried to get id of an uncertain seat"),
        }
    }
}
#[derive(Clone, PartialEq, Debug)]
enum Direction {
    LEFT,
    RIGHT,
    UP,
    DOWN,
}

fn shannon_entropy(seat: &Seat, weights: &Vec<f32>) -> f32 {
    match seat {
        Seat::Collapsed(_) => 0.0,
        Seat::Uncertain(possibilities) => {
            let sum: f32 = possibilities.iter().map(|v| weights[*v]).sum();
            let log_sum: f32 = possibilities
                .iter()
                .map(|v| weights[*v] * weights[*v].log2())
                .sum();
            sum.log2() - log_sum / sum
        }
    }
}

// (SEA, COAST, LEFT) means that SEA can exist to the LEFT of COAST
#[derive(Clone, PartialEq, Debug)]
struct Rule(usize, usize, Direction);

#[derive(Clone)]
enum GridState {
    Uncollapsed,
    Collapsed,
    Contradicting,
}

impl Grid {
    fn new(w: usize, h: usize, num_possibilities: usize) -> Grid {
        Grid {
            data: vec![vec![Seat::new(num_possibilities); w]; h],
            num_possibilities,
            w,
            h,
        }
    }
    fn get_state(&self) -> GridState {
        if self
            .data
            .iter()
            .map(|v| v.iter())
            .flatten()
            .any(|p| match p {
                Seat::Uncertain(v) => v.len() == 0,
                Seat::Collapsed(_) => false,
            })
        {
            GridState::Contradicting
        } else if self
            .data
            .iter()
            .map(|v| v.iter())
            .flatten()
            .all(|p| match p {
                Seat::Uncertain(v) => v.len() == 1,
                Seat::Collapsed(_) => true,
            })
        {
            GridState::Collapsed
        } else {
            GridState::Uncollapsed
        }
    }
    fn possibilities(
        &self,
        surrounding: &Seat,
        direction: &Direction,
        rules: &Vec<Rule>,
    ) -> Option<Vec<usize>> {
        let mut temp_possibilities = Vec::new();
        match surrounding {
            Seat::Uncertain(s) => {
                for cell_type in s.iter() {
                    for v in rules
                        .iter()
                        .filter(|v| v.1 == *cell_type && v.2 == *direction)
                        .map(|v| v.0)
                    {
                        if !temp_possibilities.contains(&v) {
                            temp_possibilities.push(v)
                        }
                    }
                }
            }
            Seat::Collapsed(cell_type) => {
                for v in rules
                    .iter()
                    .filter(|v| v.1 == *cell_type && v.2 == *direction)
                    .map(|v| v.0)
                {
                    if !temp_possibilities.contains(&v) {
                        temp_possibilities.push(v)
                    }
                }
            }
        }
        if temp_possibilities.len() == 0 {
            None
        } else {
            Some(temp_possibilities)
        }
    }
    fn step(&mut self, rules: &Vec<Rule>, weights: &Vec<f32>) {
        let mut min_entropy = (0, 0, std::f32::MAX);
        for y in 0..self.h {
            for x in 0..self.w {
                match &self.data[y][x] {
                    Seat::Collapsed(_) => {
                        continue;
                    }
                    Seat::Uncertain(v) => {
                        if v.len() == 1 {
                            self[y][x] = Seat::Collapsed(v[0])
                        }
                    }
                }
                let current_entropy = shannon_entropy(&self.data[y][x], weights);
                if current_entropy < min_entropy.2 {
                    min_entropy = (y, x, current_entropy);
                }
            }
        }
        let mut possibilties_collection = Vec::new();
        let (y, x, _) = min_entropy;
        if y != 0 {
            let up_possibilities = self.possibilities(&self.data[y - 1][x], &Direction::UP, &rules);
            if up_possibilities.is_some() {
                possibilties_collection.push(up_possibilities.unwrap());
            }
        }
        if y < self.h - 1 {
            let down_possibilities =
                self.possibilities(&self.data[y + 1][x], &Direction::DOWN, &rules);
            if down_possibilities.is_some() {
                possibilties_collection.push(down_possibilities.unwrap());
            }
        }
        if x < self.w - 1 {
            let right_possibilities =
                self.possibilities(&self.data[y][x + 1], &Direction::RIGHT, &rules);
            if right_possibilities.is_some() {
                possibilties_collection.push(right_possibilities.unwrap());
            }
        }
        if x != 0 {
            let left_possibilities =
                self.possibilities(&self.data[y][x - 1], &Direction::LEFT, &rules);
            if left_possibilities.is_some() {
                possibilties_collection.push(left_possibilities.unwrap());
            }
        }
        let mut possibilities = possibilties_collection.pop().unwrap();
        for p in possibilties_collection.iter() {
            possibilities = p
                .iter()
                .filter(|v| possibilities.contains(v))
                .map(|v| *v)
                .collect();
        }
        possibilities.sort_by(|v0, v1| weights[*v0].total_cmp(&weights[*v1]));
        let mut sampled_idx = None;
        let some_random_number: f32 = rand::random();
        for possibility in possibilities.iter() {
            if some_random_number < weights[*possibility] {
                sampled_idx = Some(possibility);
                break;
            }
        }
        if sampled_idx.is_none() {
            sampled_idx = possibilities.last();
        }
        self.data[min_entropy.0][min_entropy.1] = match sampled_idx {
            Some(v) => Seat::Collapsed(*v),
            None => Seat::Uncertain(vec![]),
        };
    }
    fn collapse(&mut self, rules: &Vec<Rule>, weights: &Vec<f32>) {
        match self.get_state() {
            GridState::Uncollapsed => {
                self.step(rules, weights);
                self.collapse(rules, weights)
            }
            GridState::Collapsed => {}
            GridState::Contradicting => {
                self.data = vec![vec![Seat::new(self.num_possibilities); self.w]; self.h];
                self.collapse(rules, weights);
            }
        }
    }
}

impl Seat {
    fn new(num_possibilities: usize) -> Seat {
        if num_possibilities > 1 || num_possibilities == 0 {
            Seat::Uncertain(Vec::from_iter(0..num_possibilities))
        } else {
            Seat::Collapsed(0)
        }
    }
}

fn generate_rules(grid: &Grid) -> (Vec<Rule>, Vec<f32>) {
    let mut rules = Vec::new();
    let mut weights: Vec<usize> = vec![0; grid.num_possibilities];
    for y in 0..(grid.h) {
        for x in 0..(grid.w) {
            match &grid[y][x] {
                Seat::Collapsed(i) => {
                    weights[*i] += 1;
                }
                Seat::Uncertain(v) => {
                    for i in v.iter() {
                        weights[*i] += 1;
                    }
                }
            }
        }
    }
    let weights = weights
        .into_iter()
        .map(|v| v as f32 / (grid.w * grid.h) as f32)
        .collect();
    for y in 1..(grid.h - 1) {
        for x in 1..(grid.w - 1) {
            let current = grid[y][x].get_id().unwrap();
            let left = grid[y][x - 1].get_id().unwrap();
            let right = grid[y][x + 1].get_id().unwrap();
            let up = grid[y + 1][x].get_id().unwrap();
            let down = grid[y - 1][x].get_id().unwrap();

            if !rules.contains(&Rule(left, current, Direction::LEFT)) {
                rules.push(Rule(left, current, Direction::LEFT))
            }
            if !rules.contains(&Rule(right, current, Direction::RIGHT)) {
                rules.push(Rule(right, current, Direction::RIGHT))
            }
            if !rules.contains(&Rule(up, current, Direction::UP)) {
                rules.push(Rule(up, current, Direction::UP))
            }
            if !rules.contains(&Rule(down, current, Direction::DOWN)) {
                rules.push(Rule(down, current, Direction::DOWN))
            }
        }
    }
    (rules, weights)
}

#[derive(Debug)]
enum TileGenError {
    NonFactorWidth,
    NonFactorHeight,
}

fn generate_tiles<'a, T: GenericImage<Pixel = P>, P: Pixel + std::cmp::PartialEq>(
    img: &'a T,
    tile_w: u32,
    tile_h: u32,
) -> Result<(Grid, Vec<SubImage<&T>>), TileGenError> {
    if img.width() % tile_w != 0 {
        return Err(TileGenError::NonFactorWidth);
    }
    if img.height() % tile_h != 0 {
        return Err(TileGenError::NonFactorHeight);
    }
    let mut grid = Grid::new(
        (img.width() / tile_w) as usize,
        (img.height() / tile_h) as usize,
        1,
    );
    let mut tiles = Vec::new();
    for y in 0..grid.h {
        for x in 0..grid.w {
            let current_tile = img.view(x as u32 * tile_w, y as u32 * tile_h, tile_w, tile_h);
            match tiles.iter().position(|v: &SubImage<&T>| {
                current_tile
                    .pixels()
                    .zip((*v).pixels())
                    .all(|((_, _, c1), (_, _, c2))| c1 == c2)
            }) {
                Some(v) => {
                    grid[y][x] = Seat::Collapsed(v);
                }
                None => {
                    grid[y][x] = Seat::Collapsed(tiles.len());
                    tiles.push(current_tile);
                    grid.num_possibilities += 1;
                }
            }
        }
    }
    Ok((grid, tiles))
}
fn main() {
    let img = image::open("test.png").unwrap();
    let (sample_grid, tiles) = generate_tiles(&img, 1, 1).unwrap();
    let (rules, weights) = generate_rules(&sample_grid);
    let mut grid = Grid::new(100, 100, tiles.len());
    let mut max_idx = 0;
    for i in 0..weights.len() {
        if weights[max_idx] < weights[i] {
            max_idx = i;
        }
    }
    grid.data[0][0] = Seat::Collapsed(max_idx);
    grid.collapse(&rules, &weights);
    grid.render(tiles).save("temp_out.png").unwrap();
}
