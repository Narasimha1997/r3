//! Module for the main struct, which marks the begin of a Multiboot2 header.
//! See [`Multiboot2Header`].

pub mod builder;

pub use self::builder::*;
use crate::{AddressHeaderTag, InformationRequestHeaderTag, RelocatableHeaderTag, StructAsBytes};
use crate::{ConsoleHeaderTag, EntryHeaderTag};
use crate::{EfiBootServiceHeaderTag, FramebufferHeaderTag};
use crate::{EndHeaderTag, HeaderTagType};
use crate::{EntryEfi32HeaderTag, EntryEfi64HeaderTag};
use crate::{HeaderTag, HeaderTagISA};
use core::fmt::{Debug, Formatter};
use core::mem::size_of;

/// Magic value for a [`Multiboot2Header`], as defined in spec.
pub const MULTIBOOT2_HEADER_MAGIC: u32 = 0xe85250d6;

/// Wrapper type around a pointer to the Multiboot2 header.
/// The Multiboot2 header is the [`Multiboot2BasicHeader`] followed
/// by all tags (see [`crate::tags::HeaderTagType`]).
/// Use this if you get a pointer to the header and just want
/// to parse it. If you want to construct the type by yourself,
/// please look at [`builder::Multiboot2HeaderBuilder`].
#[repr(transparent)]
pub struct Multiboot2Header<'a> {
    inner: &'a Multiboot2BasicHeader,
}

impl<'a> Multiboot2Header<'a> {
    /// Public constructor for this type with various validations. It panics if the address is invalid.
    /// It panics rather than returning a result, because if this fails, it is
    /// a fatal, unrecoverable error anyways and a bug in your code.
    ///
    /// # Panics
    /// Panics if one of the following conditions is true:
    /// - `addr` is a null-pointer
    /// - `addr` isn't 8-byte aligned
    /// - the magic value of the header is not present
    /// - the checksum field is invalid
    ///
    /// # Safety
    /// This function may produce undefined behaviour, if the provided `addr` is not a valid
    /// Multiboot2 header pointer.
    pub unsafe fn from_addr(addr: usize) -> Self {
        assert_ne!(0, addr, "`addr` is null pointer");
        assert_eq!(
            addr % 8,
            0,
            "`addr` must be 8-byte aligned, see Multiboot2 spec"
        );
        let ptr = addr as *const Multiboot2BasicHeader;
        let reference = &*ptr;
        assert_eq!(
            reference.header_magic(),
            MULTIBOOT2_HEADER_MAGIC,
            "The Multiboot2 header must contain the MULTIBOOT2_HEADER_MAGIC={:x}",
            MULTIBOOT2_HEADER_MAGIC
        );
        assert!(
            reference.verify_checksum(),
            "checksum invalid! Is {:x}, expected {:x}",
            reference.checksum(),
            Self::calc_checksum(reference.header_magic, reference.arch, reference.length)
        );
        Self { inner: reference }
    }

    /// Wrapper around [`Multiboot2BasicHeader::verify_checksum`].
    pub const fn verify_checksum(&self) -> bool {
        self.inner.verify_checksum()
    }
    /// Wrapper around [`Multiboot2BasicHeader::header_magic`].
    pub const fn header_magic(&self) -> u32 {
        self.inner.header_magic()
    }
    /// Wrapper around [`Multiboot2BasicHeader::arch`].
    pub const fn arch(&self) -> HeaderTagISA {
        self.inner.arch()
    }
    /// Wrapper around [`Multiboot2BasicHeader::length`].
    pub const fn length(&self) -> u32 {
        self.inner.length()
    }
    /// Wrapper around [`Multiboot2BasicHeader::checksum`].
    pub const fn checksum(&self) -> u32 {
        self.inner.checksum()
    }
    /// Wrapper around [`Multiboot2BasicHeader::tag_iter`].
    pub fn iter(&self) -> Multiboot2HeaderTagIter {
        self.inner.tag_iter()
    }
    /// Wrapper around [`Multiboot2BasicHeader::calc_checksum`].
    pub const fn calc_checksum(magic: u32, arch: HeaderTagISA, length: u32) -> u32 {
        Multiboot2BasicHeader::calc_checksum(magic, arch, length)
    }
}

impl<'a> Debug for Multiboot2Header<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        // For debug fmt we only output the inner field
        let reference = unsafe { &*(self.inner as *const Multiboot2BasicHeader) };
        Debug::fmt(reference, f)
    }
}

/// **Use this only if you know what you do. You probably want to use
/// [`Multiboot2Header`] instead.**
///
/// The "basic" Multiboot2 header. This means only the properties, that are known during
/// compile time. All other information are derived during runtime from the size property.
#[derive(Copy, Clone)]
#[repr(C, packed(8))]
pub struct Multiboot2BasicHeader {
    /// Must be the value of [`MULTIBOOT2_HEADER_MAGIC`].
    header_magic: u32,
    arch: HeaderTagISA,
    length: u32,
    checksum: u32,
    // additional tags..
    // at minimum the end tag
}

impl Multiboot2BasicHeader {
    /// Constructor for the basic header.
    pub(crate) const fn new(arch: HeaderTagISA, length: u32) -> Self {
        let magic = MULTIBOOT2_HEADER_MAGIC;
        let checksum = Self::calc_checksum(magic, arch, length);
        Multiboot2BasicHeader {
            header_magic: magic,
            arch,
            length,
            checksum,
        }
    }

    /// Verifies that a Multiboot2 header is valid.
    pub const fn verify_checksum(&self) -> bool {
        let check = Self::calc_checksum(self.header_magic, self.arch, self.length);
        check == self.checksum
    }

    /// Calculates the checksum as described in the spec.
    pub const fn calc_checksum(magic: u32, arch: HeaderTagISA, length: u32) -> u32 {
        (0x100000000 - magic as u64 - arch as u64 - length as u64) as u32
    }

    /// Returns
    pub const fn header_magic(&self) -> u32 {
        self.header_magic
    }
    pub const fn arch(&self) -> HeaderTagISA {
        self.arch
    }
    pub const fn length(&self) -> u32 {
        self.length
    }
    pub const fn checksum(&self) -> u32 {
        self.checksum
    }

    /// Returns a [`Multiboot2HeaderTagIter`].
    ///
    /// # Panics
    /// See doc of [`Multiboot2HeaderTagIter`].
    pub fn tag_iter(&self) -> Multiboot2HeaderTagIter {
        let base_hdr_size = size_of::<Multiboot2BasicHeader>();
        if base_hdr_size == self.length as usize {
            panic!("No end tag!");
        }
        let tag_base_addr = self as *const Multiboot2BasicHeader;
        // cast to u8 so that the offset in bytes works correctly
        let tag_base_addr = tag_base_addr as *const u8;
        // tag_base_addr should now point behind the "static" members
        let tag_base_addr = unsafe { tag_base_addr.add(base_hdr_size) };
        // align pointer to 8 byte according to spec
        let tag_base_addr = unsafe { tag_base_addr.add(tag_base_addr.align_offset(8)) };
        // cast back
        let tag_base_addr = tag_base_addr as *const HeaderTag;
        let tags_len = self.length as usize - base_hdr_size;
        Multiboot2HeaderTagIter::new(tag_base_addr, tags_len as u32)
    }
}

impl Debug for Multiboot2BasicHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Multiboot2Header")
            .field("header_magic", &{ self.header_magic })
            .field("arch", &{ self.arch })
            .field("length", &{ self.length })
            .field("checksum", &{ self.checksum })
            .field("tags", &self.tag_iter())
            .finish()
    }
}

impl StructAsBytes for Multiboot2BasicHeader {}

/// Iterator over all tags of a Multiboot2 header. The number of items is derived
/// by the size/length of the header.
///
/// # Panics
/// Panics if the `length`-attribute doesn't match the number of found tags, there are
/// more tags found than technically possible, or if there is more than one end tag.
/// All of these errors come from bigger, underlying problems. Therefore, they are
/// considered as "abort/panic" and not as recoverable errors.
#[derive(Clone)]
pub struct Multiboot2HeaderTagIter {
    /// 8-byte aligned base address
    base: *const HeaderTag,
    /// Offset in bytes from the base address.
    /// Always <= than size.
    n: u32,
    /// Size / final value of [`Self::n`].
    size: u32,
    /// Counts the number of found tags. If more tags are found
    /// than technically possible, for example because the length property
    /// was invalid and there are hundreds of "End"-tags, we can use
    /// this and enforce a hard iteration limit.
    tag_count: u32,
    /// Marks if the end-tag was found. Together with `tag_count`, this
    /// further helps to improve safety when invalid length properties are given.
    end_tag_found: bool,
}

impl Multiboot2HeaderTagIter {
    fn new(base: *const HeaderTag, size: u32) -> Self {
        // transform to byte pointer => offset works properly
        let base = base as *const u8;
        let base = unsafe { base.add(base.align_offset(8)) };
        let base = base as *const HeaderTag;
        Self {
            base,
            n: 0,
            size,
            tag_count: 0,
            end_tag_found: false,
        }
    }
}

impl Iterator for Multiboot2HeaderTagIter {
    type Item = *const HeaderTag;

    fn next(&mut self) -> Option<Self::Item> {
        // no more bytes left to check; length reached
        if self.n >= self.size {
            return None;
        }

        // transform to byte ptr => offset works correctly
        let ptr = self.base as *const u8;
        let ptr = unsafe { ptr.add(self.n as usize) };
        let ptr = ptr as *const HeaderTag;
        assert_eq!(ptr as usize % 8, 0, "must be 8-byte aligned");
        let tag = unsafe { &*ptr };
        assert!(
            tag.size() <= 500,
            "no real mb2 header should be bigger than 500bytes - probably wrong memory?! is: {}",
            { tag.size() }
        );
        assert!(
            tag.size() >= 8,
            "no real mb2 header tag is smaller than 8 bytes - probably wrong memory?! is: {}",
            { tag.size() }
        );
        assert!(
            !self.end_tag_found,
            "There is more than one end tag! Maybe the `length` property is invalid?"
        );
        self.n += tag.size();
        // 8-byte alignment of pointer address
        self.n += self.n % 8;
        self.tag_count += 1;
        if tag.typ() == HeaderTagType::End {
            self.end_tag_found = true;
        }
        assert!(self.tag_count < HeaderTagType::count(), "Invalid Multiboot2 header tags! There are more tags than technically possible! Maybe the `length` property is invalid?");
        Some(ptr)
    }
}

impl Debug for Multiboot2HeaderTagIter {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mut debug = f.debug_list();
        self.clone().for_each(|t| unsafe {
            let typ = (*t).typ();
            if typ == HeaderTagType::End {
                let entry = t as *const EndHeaderTag;
                let entry = &*(entry);
                debug.entry(entry);
            } else if typ == HeaderTagType::InformationRequest {
                let entry = t as *const InformationRequestHeaderTag<0>;
                let entry = &*(entry);
                debug.entry(entry);
            } else if typ == HeaderTagType::Address {
                let entry = t as *const AddressHeaderTag;
                let entry = &*(entry);
                debug.entry(entry);
            } else if typ == HeaderTagType::EntryAddress {
                let entry = t as *const EntryHeaderTag;
                let entry = &*(entry);
                debug.entry(entry);
            } else if typ == HeaderTagType::ConsoleFlags {
                let entry = t as *const ConsoleHeaderTag;
                let entry = &*(entry);
                debug.entry(entry);
            } else if typ == HeaderTagType::Framebuffer {
                let entry = t as *const FramebufferHeaderTag;
                let entry = &*(entry);
                debug.entry(entry);
            } else if typ == HeaderTagType::EfiBS {
                let entry = t as *const EfiBootServiceHeaderTag;
                let entry = &*(entry);
                debug.entry(entry);
            } else if typ == HeaderTagType::EntryAddressEFI32 {
                let entry = t as *const EntryEfi32HeaderTag;
                let entry = &*(entry);
                debug.entry(entry);
            } else if typ == HeaderTagType::EntryAddressEFI64 {
                let entry = t as *const EntryEfi64HeaderTag;
                let entry = &*(entry);
                debug.entry(entry);
            } else if typ == HeaderTagType::Relocatable {
                let entry = t as *const RelocatableHeaderTag;
                let entry = &*(entry);
                debug.entry(entry);
            } else {
                panic!("unknown tag ({:?})!", typ);
            }
        });
        debug.finish()
    }
}
