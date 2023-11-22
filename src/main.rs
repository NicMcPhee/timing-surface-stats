use std::{fs, collections::HashMap};

use anyhow::Context;
use nom::{
    bytes::complete::tag,
    character::complete::{char, u32, space0},
    combinator::map,
    sequence::{preceded, separated_pair, tuple},
    IResult, branch::alt, number::complete::float,
};

// fn parse_results(input: &str) -> IResult<&str, Summary> {
//     let (input, _) = tag("#")(input)?;
//     let (input, (red, green, blue)) = (hex_primary, hex_primary, hex_primary).parse(input)?;
//     Ok((input, Color { red, green, blue }))
//   }

#[derive(Debug, Default)]
struct Result {
    num_runs: usize,
    num_successes: usize,
    run_times: Vec<f32>,
}

type Data = HashMap<(u32, u32), Result>;

#[derive(Debug)]
struct Stat {
    result: Result,
    mean_run_time: f32,
    median_run_time: f32,
    successes_per_mean: f32,
    successes_per_median: f32,
}

type Stats = HashMap<(u32, u32), Stat>;

#[derive(Debug)]
enum Entry {
    Success,
    RunTime(f32),
}

#[derive(Debug)]
struct Line {
    population_size: u32,
    num_generations: u32,
    run_number: u32,
    entry: Entry,
}

impl<'a> FromIterator<&'a Line> for Data {
    fn from_iter<T: IntoIterator<Item = &'a Line>>(iter: T) -> Self {
        let mut data = Self::new();
        for line in iter {
            let key = (line.population_size, line.num_generations);
            let result = data.entry(key).or_default();
            match line.entry {
                Entry::Success => {
                    result.num_successes += 1;
                },
                Entry::RunTime(value) => {
                    result.num_runs += 1;
                    result.run_times.push(value);
                },
            }
        }
        data
    }
}

fn median(vals: &mut[f32]) -> f32 {
    vals.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    if vals.len() % 2 == 1 {
        vals[vals.len() / 2]
    } else {
        (vals[vals.len() / 2 - 1] + vals[vals.len() / 2]) / 2.0
    }
}

fn mean(vals: &[f32]) -> f32 {
    vals.iter().sum::<f32>() / vals.len() as f32
}

fn data_to_stats(data: Data) -> Stats {
    let mut stats = Stats::new();
    for (key, result) in data {
        let successes = result.num_successes as f32;
        let mut run_times = result.run_times.clone();
        let mean_run_time = mean(&run_times);
        let median_run_time = median(&mut run_times);
        stats.insert(key, Stat { result, mean_run_time, median_run_time, successes_per_mean: successes / mean_run_time, successes_per_median: successes / median_run_time });
    }
    stats
}

fn pop_size(s: &str) -> IResult<&str, u32> {
    preceded(tag("PS_"), u32)(s)
}

fn num_gens(s: &str) -> IResult<&str, u32> {
    preceded(tag("/NG_"), u32)(s)
}

fn run_num(s: &str) -> IResult<&str, u32> {
    // run_29.output
    map(
        tuple((tag("/run_"), u32, tag(".output"))),
        |(_, run_num, _)| run_num,
    )(s)
}

fn path(s: &str) -> IResult<&str, (u32, u32, u32)> {
    tuple((pop_size, num_gens, run_num))(s)
}

fn success(s: &str) -> IResult<&str, Entry> {
    map(tag("SUCCESS"), |_| Entry::Success)(s)
}

fn run_time(s: &str) -> IResult<&str, Entry> {
    map(preceded(space0, float), |run_time| Entry::RunTime(run_time))(s)
}

fn entry(s: &str) -> IResult<&str, Entry> {
    alt((success, run_time))(s)
}

fn line(s: &str) -> IResult<&str, Line> {
    map(
        separated_pair(path, char(':'), entry),
        |((population_size, num_generations, run_number), entry)| Line {
            population_size,
            num_generations,
            run_number,
            entry,
        },
    )(s)
}

fn parse_line(s: &str) -> anyhow::Result<Line> {
    let (_, l) = line(s).map_err(nom::Err::<nom::error::Error<&str>>::to_owned)?;
    Ok(l)
}

fn main() -> anyhow::Result<()> {
    let path = "../all_runs.output";

    let lines = fs::read_to_string(path)
        .with_context(|| format!("Couldn't open file {path}"))?
        .lines()
        .map(parse_line)
        .collect::<anyhow::Result<Vec<_>>>()?;

    let data: Data = lines.iter().collect();
    let stats = data_to_stats(data);

    println!("{stats:?}");

    println!();
    println!("PopSize   NumGens SuccessesPerMean");
    let mut pairs = stats.iter().collect::<Vec<(_, _)>>();
    pairs.sort_unstable_by(|(_, b), (_, y)| b.successes_per_mean.partial_cmp(&y.successes_per_mean).unwrap());
    for ((pop_size, num_gens), s) in &pairs {
        println!("{pop_size}    {num_gens}  {}", s.successes_per_mean);
    }

    println!();
    println!("PopSize   NumGens SuccessesPerMedian");
    pairs.sort_unstable_by(|(_, b), (_, y)| b.successes_per_median.partial_cmp(&y.successes_per_median).unwrap());
    for ((pop_size, num_gens), s) in &pairs {
        println!("{pop_size}    {num_gens}  {}", s.successes_per_median);
    }

    Ok(())
}
