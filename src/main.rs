use image::{GenericImage, Pixel, Rgb};

struct Grid {
    data: Vec<Vec<Seat>>,
    num_possibilities: usize,
    w: usize,
    h: usize,
}
#[derive(Clone, Debug)]
struct Seat(Vec<usize>);

#[derive(Clone, PartialEq, Debug)]
enum Direction {
    LEFT,
    RIGHT,
    UP,
    DOWN,
}

fn shannon_entropy(seat: &Seat, weights: &Vec<f32>) -> f32 {
    let sum: f32 = seat.0.iter().map(|v| weights[*v]).sum();
    let log_sum: f32 = seat
        .0
        .iter()
        .map(|v| weights[*v] * weights[*v].log2())
        .sum();
    sum.log2() - log_sum / sum
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
            .any(|p| p.0.len() == 0)
        {
            GridState::Contradicting
        } else if self
            .data
            .iter()
            .map(|v| v.iter())
            .flatten()
            .all(|p| p.0.len() == 1)
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
    ) -> Vec<usize> {
        let mut temp_possibilities = Vec::new();
        for cell_type in surrounding.0.iter() {
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
        temp_possibilities
    }
    fn step(&mut self, rules: &Vec<Rule>, weights: &Vec<f32>) {
        let mut min_entropy = (0, 0, std::f32::MAX);
        for y in 0..self.h {
            for x in 0..self.w {
                if self.data[y][x].0.len() == 1 {
                    continue;
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
            possibilties_collection.push(up_possibilities);
        }
        if y < self.h - 1 {
            let down_possibilities =
                self.possibilities(&self.data[y + 1][x], &Direction::DOWN, &rules);
            possibilties_collection.push(down_possibilities);
        }
        if x < self.w - 1 {
            let right_possibilities =
                self.possibilities(&self.data[y][x + 1], &Direction::RIGHT, &rules);
            possibilties_collection.push(right_possibilities);
        }
        if x != 0 {
            let left_possibilities =
                self.possibilities(&self.data[y][x - 1], &Direction::LEFT, &rules);
            possibilties_collection.push(left_possibilities);
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
        self.data[min_entropy.0][min_entropy.1] = Seat(vec![*sampled_idx.expect("Something should have been sampled")]);
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
        Seat(Vec::from_iter(0..num_possibilities))
    }
}

fn generate_rules<T: GenericImage<Pixel = P>, P: Pixel + std::cmp::PartialEq>(
    img: T,
) -> (Vec<Rule>, Vec<f32>, Vec<P>) {
    let mut rules = Vec::new();
    let mut colors = Vec::new();
    let mut weights: Vec<usize> = Vec::new();
    for x in 0..(img.width()) {
        for y in 0..(img.height()) {
            let a = img.get_pixel(x, y);
            match colors.iter().position(|v| *v == a) {
                None => {
                    colors.push(a);
                    weights.push(1);
                }
                Some(i) => {
                    weights[i] += 1;
                }
            };
            if !colors.contains(&a) {
                colors.push(a);
            }
        }
    }
    let weights = weights
        .into_iter()
        .map(|v| v as f32 / colors.len() as f32)
        .collect();
    for x in 1..(img.width() - 1) {
        for y in 1..(img.height() - 1) {
            let current = img.get_pixel(x, y);
            let current_id = colors.iter().position(|v| *v == current).unwrap();
            let left = img.get_pixel(x - 1, y);
            let left_id = colors.iter().position(|v| *v == left).unwrap();
            let right = img.get_pixel(x + 1, y);
            let right_id = colors.iter().position(|v| *v == right).unwrap();
            let up = img.get_pixel(x, y + 1);
            let up_id = colors.iter().position(|v| *v == up).unwrap();
            let down = img.get_pixel(x, y - 1);
            let down_id = colors.iter().position(|v| *v == down).unwrap();

            if !rules.contains(&Rule(left_id, current_id, Direction::LEFT)) {
                rules.push(Rule(left_id, current_id, Direction::LEFT))
            }
            if !rules.contains(&Rule(right_id, current_id, Direction::RIGHT)) {
                rules.push(Rule(right_id, current_id, Direction::RIGHT))
            }
            if !rules.contains(&Rule(up_id, current_id, Direction::UP)) {
                rules.push(Rule(up_id, current_id, Direction::UP))
            }
            if !rules.contains(&Rule(down_id, current_id, Direction::DOWN)) {
                rules.push(Rule(down_id, current_id, Direction::DOWN))
            }
        }
    }
    (rules, weights, colors)
}

fn main() {
    let (rules, weights, colors) = generate_rules(image::open("test.png").unwrap());
    let mut grid = Grid::new(200, 200, colors.len());
    grid.data[0][0] = Seat(vec![0]);
    grid.collapse(&rules, &weights);
    let buffer = grid
        .data
        .iter()
        .map(|v| v.iter())
        .flatten()
        .map(|v| v.0.first().unwrap())
        .map(|v| colors[*v].0)
        .flatten()
        .collect();
    let a = image::RgbaImage::from_vec(grid.w as u32, grid.h as u32, buffer).unwrap();
    a.save("temp_out.png").unwrap();
}
