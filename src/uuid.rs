use std::fmt::Display;
use std::fs::OpenOptions;
use std::io::{Read, Write, Seek, SeekFrom};
use std::{fs, path::Path};

use fs4::FileExt;

pub struct UUID {
    pub id: String,
}

impl Display for UUID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl UUID {
    pub fn new(id: String) -> Self {
        Self { id }
    }
}

struct UUIDGeneratorState {
    node_id: u64,
    last_timestamp: u64,
    sequence: u16,
    state_file_handle: Option<fs::File>,
}

pub struct UUIDGenerator {
    state: UUIDGeneratorState,
}

impl UUIDGenerator {
    pub fn new(global_state_location: String) -> Self {
        let global_state = Self::get_global_state_from_stable_storage(&global_state_location);

        Self {
            state: global_state,
        }
    }

    fn initialize_global_state(file_handle: fs::File) -> UUIDGeneratorState {
        let time_in_100_nanosecond_intervals = Self::get_current_time_as_nanosecond_intervals();
        let current_node_id = Self::get_node_id();
        let clock_sequence = rand::random::<u16>();

        UUIDGeneratorState {
            node_id: current_node_id,
            last_timestamp: time_in_100_nanosecond_intervals,
            sequence: clock_sequence,
            state_file_handle: Some(file_handle),
        }
    }

    fn get_current_time_as_nanosecond_intervals() -> u64 {
        let current_timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap() as u64;
        let timestamp_on_epoch =
            chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00.000000000Z").unwrap();
        let timestamp_on_epoch = timestamp_on_epoch.timestamp_nanos_opt().unwrap() as u64;

        (current_timestamp - timestamp_on_epoch) / 100
    }

    fn get_global_state_from_stable_storage(global_state_location: &str) -> UUIDGeneratorState {
        let path = Path::new(global_state_location);
        let mut file_options = OpenOptions::new();
        file_options.create(true).read(true).write(true);

        let mut f = file_options.open(path).unwrap();

        // establish a global lock on this file while reading the data
        f.lock_exclusive().unwrap();

        let mut global_state = String::new();
        f.read_to_string(&mut global_state).unwrap();

        /*
         if the global state is empty, we initialize a new state,
         write that to the file, release the lock and then return the state
        */
        if global_state.is_empty() {
            let state = Self::initialize_global_state(f);
            global_state = format!(
                "{}\n{}\n{}",
                state.last_timestamp, state.sequence, state.node_id
            );

            let mut file_handle = state.state_file_handle.as_ref().unwrap();
            file_handle.seek(SeekFrom::Start(0)).unwrap();
            state.state_file_handle.as_ref().unwrap().write_fmt(format_args!("{}", global_state)).unwrap();
            return state;
        }

        /*
         otherwise, fetch the state from the shared file and return it
        */
        let state = global_state
            .split('\n')
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        let last_timestamp = state[0].parse::<u64>().unwrap();
        let mut last_sequence_id = state[1].parse::<u16>().unwrap();
        let node_id = Self::get_node_id();
        let current_timestamp = Self::get_current_time_as_nanosecond_intervals();

        if last_timestamp > current_timestamp {
            last_sequence_id += 1;
        }

        UUIDGeneratorState {
            node_id,
            last_timestamp: current_timestamp,
            sequence: last_sequence_id,
            state_file_handle: Some(f),
        }
    }

    fn get_node_id() -> u64 {
        let net = Path::new("/sys/class/net");
        let entry = fs::read_dir(net).unwrap();

        /*
         * On Unix-like systems, /sys/class/net/ contains the symlinks to the available interfaces. The MAC address of an interface
         * is written in a file like /sys/class/net/eth0/address
         *
         * ref: https://stackoverflow.com/questions/26346575/how-to-get-mac-address-in-rust
         */
        let ifaces = entry
            .filter_map(|p| p.ok())
            .map(|p| p.path().file_name().unwrap().to_os_string())
            .filter_map(|s| s.into_string().ok())
            .collect::<Vec<String>>();

        let iface = net.join(ifaces[1].as_str()).join("address");
        let mut f = fs::File::open(iface).unwrap();
        let mut mac_address = String::new();
        f.read_to_string(&mut mac_address).unwrap();

        mac_address
            .as_bytes()
            .iter()
            .fold(0, |acc, &byte| (acc << 8) + byte as u64)
    }

    /*
      generate time_low

      we create a bitmask 0xFFFF_FFFF
      which is 32 bits of 1s

      when trying to AND it with a u64, it gets padded with 0s in the front, making it 64 bits long
      on executing the AND, we get the 32 least significant bits of the u64
    */
    fn get_time_low(&mut self) -> u32 {
        let current_timestamp = self.state.last_timestamp;
        let lsb_32_bits_mask = 0xFFFF_FFFF;

        (current_timestamp & lsb_32_bits_mask) as u32
    }

    /*
      timestamp is (for eg.) 0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111
      we need bits 32 through 47

      we shift it by 32 bits, giving us bits 32 through 63
      0b1111_1111_1111_1111_1111_1111_1111_1111

      now, the mask we need is 0b1111_1111_1111_1111
      we AND it with the shifted timestamp, giving us bits 32 through 47
    */
    fn get_time_mid(&mut self) -> u16 {
        let current_timestamp = self.state.last_timestamp;
        let shifted_timestamp = current_timestamp >> 32;
        let mid_16_bits_mask = 0xFFFF;

        (shifted_timestamp & mid_16_bits_mask) as u16
    }

    fn get_time_hi_and_version(&mut self) -> u16 {
        let current_timestamp = self.state.last_timestamp;
        let mut time_hi_and_version = 0b0000_1111_1111_1111;

        /*
          we need timestamp bits 48 through 59
          we shift the timestamp by 48 bits, giving us bits 48 through 63
        */
        let shifted_timestamp = current_timestamp >> 48;
        let time_hi_mask = 0b0000_1111_1111_1111;
        let required_timestamp_bits = (shifted_timestamp & time_hi_mask) as u16;

        /* put bits 48-59 into the end of time_and_hi_version */
        time_hi_and_version &= required_timestamp_bits;

        /* 0100 is version 4 - which is what we want to generate */
        let version: u16 = 0b0100_0000_0000_0000;

        time_hi_and_version | version
    }

    fn get_clock_seq_low(&mut self) -> u8 {
        self.state.sequence as u8
    }

    fn get_clock_seq_hi_and_reserved(&mut self) -> u8 {
        let mut clock_seq_hi_and_reserved = 0b0011_1111;

        /*
          we need bits 8 through 13 of the sequence
          we shift the sequence by 8 bits, giving us bits 8 through 15
        */
        let shifted_sequence = self.state.sequence >> 8;
        let clock_seq_hi_mask = 0b0011_1111;
        let required_sequence_bits = (shifted_sequence & clock_seq_hi_mask) as u8;

        /* put bits 8-13 into the end of clock_seq_hi_and_reserved */
        clock_seq_hi_and_reserved &= required_sequence_bits;

        /* 10 is reserved for UUIDs generated by this algorithm */
        let reserved: u8 = 0b1000_0000;

        clock_seq_hi_and_reserved | reserved
    }

    fn commit_state_and_release_lock(&mut self) {
        let f = self.state.state_file_handle.as_mut().unwrap();
        let global_state = format!(
            "{}\n{}\n{}",
            self.state.last_timestamp, self.state.sequence, self.state.node_id
        );

        f.seek(SeekFrom::Start(0)).unwrap();
        f.write_fmt(format_args!("{}", global_state)).unwrap();
        f.flush().unwrap();
        f.unlock().unwrap();
    }

    /**
       UUID layout
        0                   1                   2                   3
         0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
       +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
       |                          time_low                             |
       +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
       |       time_mid                |         time_hi_and_version   |
       +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
       |clk_seq_hi_res |  clk_seq_low  |         node (0-1)            |
       +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
       |                         node (2-5)                            |
       +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
     *
    */
    pub fn generate(&mut self) -> UUID {
        let mut uuid: u128 = 0;

        let time_low = self.get_time_low();
        let time_mid = self.get_time_mid();
        let time_hi_and_version = self.get_time_hi_and_version();
        let clock_seq_hi_and_reserved = self.get_clock_seq_hi_and_reserved();
        let clock_seq_low = self.get_clock_seq_low();
        let node_id = UUIDGenerator::get_node_id();

        uuid |= (time_low as u128) << 96;
        uuid |= (time_mid as u128) << 80;
        uuid |= (time_hi_and_version as u128) << 64;
        uuid |= (clock_seq_hi_and_reserved as u128) << 56;
        uuid |= (clock_seq_low as u128) << 48;
        uuid |= (node_id as u128) << 16;

        let uuid = UUID::new(uuid.to_string());

        self.commit_state_and_release_lock();
        uuid
    }
}
