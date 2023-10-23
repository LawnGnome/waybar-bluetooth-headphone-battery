use std::{collections::BTreeSet, str::FromStr};

use clap::Parser;
use humantime::Duration;
use num::FromPrimitive;
use num_derive::FromPrimitive;
use serde::Serialize;
use strum::{EnumString, EnumVariantNames, VariantNames};
use textwrap::Options;
use tokio::select;
use tokio_stream::StreamExt;
use upower_dbus::{DeviceProxy, UPowerProxy};
use zbus::Connection;

#[derive(Debug, Parser)]
struct Opt {
    /// Bluetooth device kinds to match.
    #[arg(short, long, default_value = "headset, headphones", long_help = DeviceKindSet::long_help())]
    kinds: DeviceKindSet,

    /// CSS class returned when the battery percentage is below --low-percentage.
    #[arg(long, default_value = "low")]
    low_class: String,

    /// The percentage below which --low-class is included in output.
    #[arg(short, long, default_value = "20")]
    low_percentage: f64,

    /// If set, run continuously.
    #[arg(long)]
    listen: bool,

    /// How frequently to refresh even if there aren't any upower events.
    #[arg(short, long, default_value = "15s")]
    refresh: Duration,
}

impl Opt {
    fn output(&self, percentage: f64, model: &str) -> WaybarOutput {
        WaybarOutput {
            text: format!("{percentage}%"),
            tooltip: Some(model.to_string()),
            class: if percentage <= self.low_percentage {
                Some(self.low_class.clone())
            } else {
                None
            },
            percentage: Some(percentage),
        }
    }
}

#[derive(Serialize)]
struct WaybarOutput {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tooltip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    percentage: Option<f64>,
}

// The upower dbus spec only includes items up to Phone, and that's what upower_dbus implements,
// but upower itself actually includes another 20 or so items.
#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    FromPrimitive,
    EnumString,
    EnumVariantNames,
)]
#[strum(serialize_all = "kebab-case")]
enum DeviceKind {
    #[default]
    Unknown = 0,
    LinePower = 1,
    Battery = 2,
    Ups = 3,
    Monitor = 4,
    Mouse = 5,
    Keyboard = 6,
    Pda = 7,
    Phone = 8,
    MediaPlayer = 9,
    Tablet = 10,
    Computer = 11,
    GamingInput = 12,
    Pen = 13,
    Touchpad = 14,
    Modem = 15,
    Network = 16,
    Headset = 17,
    Speakers = 18,
    Headphones = 19,
    Video = 20,
    OtherAudio = 21,
    RemoteControl = 22,
    Printer = 23,
    Scanner = 24,
    Camera = 25,
    Wearable = 26,
    Toy = 27,
    Generic = 28,
    Last = 29,
}

#[derive(Clone, Debug)]
struct DeviceKindSet(BTreeSet<DeviceKind>);

impl DeviceKindSet {
    fn contains(&self, kind: DeviceKind) -> bool {
        self.0.contains(&kind)
    }

    fn long_help() -> String {
        format!(
            "Bluetooth device kinds to match, comma separated. Possible values:\n\n{}",
            textwrap::fill(
                &DeviceKind::VARIANTS.join(", "),
                // We have to handle clap indenting here.
                Options::new(textwrap::termwidth() - 12)
            ),
        )
    }
}

impl FromStr for DeviceKindSet {
    type Err = strum::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut set = BTreeSet::new();
        for term in s.split(',') {
            set.insert(DeviceKind::from_str(term.trim())?);
        }

        Ok(Self(set))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Opt::try_parse()?;

    let conn = Connection::system().await?;
    let upower = UPowerProxy::new(&conn).await?;

    output_devices(&opt, &conn, &upower).await?;
    if opt.listen {
        let mut ctrl_c = Box::pin(tokio::signal::ctrl_c());
        let mut signal_stream = upower.receive_all_signals().await?;

        loop {
            let refresh = tokio::time::sleep(opt.refresh.into());

            select! {
                _signal = signal_stream.next() => output_devices(&opt, &conn, &upower).await?,
                _time = refresh => output_devices(&opt, &conn, &upower).await?,
                _ = &mut ctrl_c => break,
            };
        }
    }

    Ok(())
}

async fn output_devices(
    opt: &Opt,
    conn: &Connection,
    upower: &UPowerProxy<'_>,
) -> anyhow::Result<()> {
    let mut devices_seen = 0;

    for device in upower.enumerate_devices().await?.into_iter() {
        let proxy = DeviceProxy::new(conn, device).await?;
        let kind = DeviceKind::from_u32(proxy.get_property("Type").await?).unwrap_or_default();
        let model = proxy.model().await?;
        if opt.kinds.contains(kind) {
            devices_seen += 1;
            let output = opt.output(proxy.percentage().await?, &model);
            println!("{}", serde_json::to_string(&output)?);
        }
    }

    if devices_seen == 0 {
        println!();
    }

    Ok(())
}
