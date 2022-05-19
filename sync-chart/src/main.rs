use std::cmp::{max, min};
use std::fs::File;
use std::io::BufRead;
use std::process::exit;
use std::{env, io};

use chrono::offset::{Local, TimeZone};
use chrono::DateTime;
use plotters::chart::ChartBuilder;
use plotters::prelude::{BitMapBackend, IntoDrawingArea, IntoFont, LineSeries, BLUE, RED};
use plotters::style::WHITE;

fn parse_time(t: &str) -> DateTime<Local> {
    Local.datetime_from_str(t, "%Y-%m-%dT%H:%M:%S").unwrap()
}

fn read_parse(
    file_name: &str,
    timestamp: &mut Vec<DateTime<Local>>,
    cur_tip: &mut Vec<u32>,
    best_shared: &mut Vec<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(file_name)?;
    let lines = io::BufReader::new(file).lines();
    for line in lines.flatten() {
        // 2022-09-01T12:04:03|1234|4523
        let cols: Vec<&str> = line.split('|').collect();
        if cols.len() == 3 {
            timestamp.push(parse_time(*cols.get(0).unwrap()));
            cur_tip.push((*cols.get(1).unwrap()).parse::<u32>().unwrap());
            best_shared.push((*cols.get(2).unwrap()).parse::<u32>().unwrap());
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut timestamp: Vec<_> = vec![];
    let mut cur_tip: Vec<u32> = vec![];
    let mut best_shared: Vec<u32> = vec![];

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("accept one filename as input parameter!");
        exit(-1);
    }

    read_parse(
        args.get(1).unwrap(),
        &mut timestamp,
        &mut cur_tip,
        &mut best_shared,
    )?;

    let (x_min, x_max) = (
        *timestamp.get(0).unwrap(),
        *timestamp.last().unwrap(),
    );

    let (y_min, y_max) = (
        min(*cur_tip.first().unwrap(), *best_shared.first().unwrap()),
        max(*cur_tip.last().unwrap(), *best_shared.last().unwrap()),
    );

    let root = BitMapBackend::new("./sync_chart.png", (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .caption("sync_tip_best_shared", ("sans-serif", 40).into_font())
        .build_cartesian_2d(x_min..x_max, y_min..y_max)?;
    chart
        .configure_mesh()
        .x_desc("Timestamp")
        .y_desc("Height")
        .y_labels(timestamp.len())
        .x_label_formatter(&|v| v.format("%H:%M:%S").to_string())
        .draw()?;

    let mut cur_tip_time = vec![];
    for (time, tip) in timestamp.iter().zip(cur_tip.iter()) {
        cur_tip_time.push((*time, *tip));
    }
    let mut best_shard_time = vec![];
    for (time, tip) in timestamp.iter().zip(best_shared.iter()) {
        best_shard_time.push((*time, *tip));
    }

    chart
        .draw_series(LineSeries::new(
            cur_tip_time.iter().map(|x| (x.0, x.1)),
            &RED,
        ))?
        .label("current_tip");

    chart
        .draw_series(LineSeries::new(
            best_shard_time.iter().map(|x| (x.0, x.1)),
            &BLUE,
        ))?
        .label("best_shared");

    Ok(())
}
