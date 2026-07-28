use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::{bail, Context, Result};
use csv::{ReaderBuilder, Trim};

use crate::config::PlotXAxisMode;

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedPlotData {
    pub x_axis_mode: PlotXAxisMode,
    pub has_point_axis: bool,
    pub has_time_axis: bool,
    pub series: Vec<ImportedPlotSeries>,
    pub row_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedPlotSeries {
    pub name: String,
    pub points: Vec<ImportedPlotPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImportedPlotPoint {
    pub row_index: usize,
    pub index: Option<f64>,
    pub elapsed_secs: Option<f64>,
    pub value: f64,
}

pub fn read_plot_csv(path: &Path, max_points: usize) -> Result<ImportedPlotData> {
    let file =
        File::open(path).with_context(|| format!("无法打开历史数据文件 {}", path.display()))?;
    parse_plot_csv(file, max_points)
}

pub(crate) fn parse_plot_csv(reader: impl Read, max_points: usize) -> Result<ImportedPlotData> {
    let mut reader = ReaderBuilder::new().trim(Trim::All).from_reader(reader);
    let headers = reader.headers().context("无法读取 CSV 表头")?.clone();

    let x_axis_mode = parse_x_axis_header(headers.get(0).unwrap_or_default())?;
    let secondary_x_axis_mode = headers
        .get(1)
        .and_then(parse_tagged_x_axis_header)
        .filter(|mode| *mode != x_axis_mode);
    let series_start = if secondary_x_axis_mode.is_some() {
        2
    } else {
        1
    };
    if headers.len() <= series_start {
        bail!("CSV 至少需要一列 X 轴和一列曲线数据");
    }

    let point_column = x_axis_column(PlotXAxisMode::Point, x_axis_mode, secondary_x_axis_mode);
    let time_column = x_axis_column(PlotXAxisMode::Time, x_axis_mode, secondary_x_axis_mode);
    let mut series = headers
        .iter()
        .skip(series_start)
        .enumerate()
        .map(|(index, name)| {
            let name = name.trim();
            if name.is_empty() {
                bail!("第 {} 列曲线名称为空", index + series_start + 1);
            }
            Ok(ImportedPlotSeries {
                name: name.to_owned(),
                points: Vec::new(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let mut row_count = 0usize;
    for (record_index, record) in reader.records().enumerate() {
        let csv_row = record_index + 2;
        let record = record.with_context(|| format!("第 {csv_row} 行不是有效的 CSV 数据"))?;
        if record.iter().all(|cell| cell.trim().is_empty()) {
            continue;
        }
        if row_count >= max_points {
            bail!("CSV 数据超过 {max_points} 行的绘图上限");
        }

        let index = point_column
            .map(|column| {
                parse_number(
                    record.get(column).unwrap_or_default(),
                    csv_row,
                    headers.get(column).unwrap_or("点数"),
                )
            })
            .transpose()?;
        let elapsed_secs = time_column
            .map(|column| {
                parse_number(
                    record.get(column).unwrap_or_default(),
                    csv_row,
                    headers.get(column).unwrap_or("时间(s)"),
                )
            })
            .transpose()?;
        let mut has_value = false;
        for (series_index, imported_series) in series.iter_mut().enumerate() {
            let cell = record
                .get(series_index + series_start)
                .unwrap_or_default()
                .trim();
            if cell.is_empty() {
                continue;
            }

            imported_series.points.push(ImportedPlotPoint {
                row_index: row_count,
                index,
                elapsed_secs,
                value: parse_number(cell, csv_row, &imported_series.name)?,
            });
            has_value = true;
        }

        if !has_value {
            bail!("第 {csv_row} 行没有任何曲线数据");
        }
        row_count += 1;
    }

    if row_count == 0 {
        bail!("CSV 中没有可绘制的数据");
    }
    series.retain(|imported_series| !imported_series.points.is_empty());
    if series.is_empty() {
        bail!("CSV 中没有可绘制的曲线");
    }

    Ok(ImportedPlotData {
        x_axis_mode,
        has_point_axis: point_column.is_some(),
        has_time_axis: time_column.is_some(),
        series,
        row_count,
    })
}

fn parse_x_axis_header(header: &str) -> Result<PlotXAxisMode> {
    parse_any_x_axis_header(header).ok_or_else(|| {
        let header = header.trim().trim_start_matches('\u{feff}');
        anyhow::anyhow!(
            "无法识别 X 轴表头“{header}”，应为“点数”、“时间(s)”或带“X轴:”前缀的新版表头"
        )
    })
}

fn parse_any_x_axis_header(header: &str) -> Option<PlotXAxisMode> {
    match header.trim().trim_start_matches('\u{feff}') {
        "点数" | "X轴:点数" => Some(PlotXAxisMode::Point),
        "时间(s)" | "X轴:时间(s)" => Some(PlotXAxisMode::Time),
        _ => None,
    }
}

fn parse_tagged_x_axis_header(header: &str) -> Option<PlotXAxisMode> {
    match header.trim() {
        "X轴:点数" => Some(PlotXAxisMode::Point),
        "X轴:时间(s)" => Some(PlotXAxisMode::Time),
        _ => None,
    }
}

fn x_axis_column(
    requested: PlotXAxisMode,
    primary: PlotXAxisMode,
    secondary: Option<PlotXAxisMode>,
) -> Option<usize> {
    if primary == requested {
        Some(0)
    } else if secondary == Some(requested) {
        Some(1)
    } else {
        None
    }
}

fn parse_number(cell: &str, row: usize, column: &str) -> Result<f64> {
    let value = cell
        .trim()
        .parse::<f64>()
        .with_context(|| format!("第 {row} 行“{column}”不是有效数字"))?;
    if !value.is_finite() {
        bail!("第 {row} 行“{column}”必须是有限数字");
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::parse_plot_csv;
    use crate::config::PlotXAxisMode;

    #[test]
    fn imports_previously_exported_point_data() {
        let csv = "点数,温度,湿度\n0,12.5,60\n1,13.25,\n2,,61.5\n";
        let data = parse_plot_csv(csv.as_bytes(), 10).unwrap();

        assert_eq!(data.x_axis_mode, PlotXAxisMode::Point);
        assert!(data.has_point_axis);
        assert!(!data.has_time_axis);
        assert_eq!(data.row_count, 3);
        assert_eq!(data.series[0].name, "温度");
        assert_eq!(data.series[0].points.len(), 2);
        assert_eq!(data.series[0].points[1].row_index, 1);
        assert_eq!(data.series[1].points[1].index, Some(2.0));
        assert_eq!(data.series[1].points[1].elapsed_secs, None);
        assert_eq!(data.series[1].points[1].value, 61.5);
    }

    #[test]
    fn imports_legacy_time_data_with_quoted_series_name() {
        let csv = "时间(s),\"温度, A\"\n0.000,12.5\n0.250,13\n";
        let data = parse_plot_csv(csv.as_bytes(), 10).unwrap();

        assert_eq!(data.x_axis_mode, PlotXAxisMode::Time);
        assert!(!data.has_point_axis);
        assert!(data.has_time_axis);
        assert_eq!(data.series[0].name, "温度, A");
        assert_eq!(data.series[0].points[1].index, None);
        assert_eq!(data.series[0].points[1].elapsed_secs, Some(0.25));
    }

    #[test]
    fn imports_dual_axis_data_and_preserves_primary_axis() {
        let csv = "X轴:时间(s),X轴:点数,温度\n0.125,10,12.5\n0.250,11,13\n";
        let data = parse_plot_csv(csv.as_bytes(), 10).unwrap();

        assert_eq!(data.x_axis_mode, PlotXAxisMode::Time);
        assert!(data.has_point_axis);
        assert!(data.has_time_axis);
        assert_eq!(data.series[0].points[1].index, Some(11.0));
        assert_eq!(data.series[0].points[1].elapsed_secs, Some(0.25));
    }

    #[test]
    fn legacy_series_named_like_an_axis_is_not_misclassified() {
        let csv = "点数,时间(s),温度\n0,12.5,20\n1,13,21\n";
        let data = parse_plot_csv(csv.as_bytes(), 10).unwrap();

        assert!(data.has_point_axis);
        assert!(!data.has_time_axis);
        assert_eq!(data.series[0].name, "时间(s)");
        assert_eq!(data.series[1].name, "温度");
    }

    #[test]
    fn rejects_unknown_x_axis_header() {
        let error = parse_plot_csv("x,ch1\n0,1\n".as_bytes(), 10).unwrap_err();

        assert!(error.to_string().contains("无法识别 X 轴表头"));
    }

    #[test]
    fn rejects_data_over_the_plot_limit() {
        let error = parse_plot_csv("点数,ch1\n0,1\n1,2\n".as_bytes(), 1).unwrap_err();

        assert!(error.to_string().contains("超过 1 行"));
    }
}
