extern crate rand;
extern crate rand_xoshiro;

use crate::system::filesystem::devfs::{DevFSDescriptor, DevOps};
use crate::system::filesystem::{FSError, SeekType};
use crate::system::timer;

use rand::{RngCore, SeedableRng};
use rand_xoshiro::Xoroshiro128Plus;

pub struct SystemRandomDevice {}

impl SystemRandomDevice {
    pub fn empty() -> Self {
        SystemRandomDevice {}
    }

    #[inline]
    fn get_random_seed() -> u64 {
        // take current ticks:
        let timeval = timer::PosixTimeval::from_ticks();
        timeval.tv_usec as u64
    }

    #[inline]
    pub fn fill_bytes(&self, buffer: &mut [u8]) {
        let seed = Self::get_random_seed();
        // use chacha random generator
        let mut generator = Xoroshiro128Plus::seed_from_u64(seed);
        generator.fill_bytes(buffer);
    }
}

pub struct RandomIODriver;

impl RandomIODriver {
    pub fn empty() -> Self {
        RandomIODriver {}
    }
}

impl DevOps for RandomIODriver {
    fn read(&self, _fd: &mut DevFSDescriptor, buffer: &mut [u8]) -> Result<usize, FSError> {
        let rand_device = SystemRandomDevice::empty();
        rand_device.fill_bytes(buffer);
        Ok(buffer.len())
    }

    fn write(&self, _fd: &mut DevFSDescriptor, _buffer: &[u8]) -> Result<usize, FSError> {
        // stub
        Ok(0)
    }

    fn ioctl(&self, _command: usize, _arg: usize) -> Result<usize, FSError> {
        // stub
        Ok(0)
    }

    fn seek(&self, _fd: &mut DevFSDescriptor, _offset: u32, _st: SeekType) -> Result<u32, FSError> {
        Ok(0)
    }
}

// TODO: Find a best way to mitigate this
unsafe impl Sync for RandomIODriver {}
unsafe impl Send for RandomIODriver {}
