#[repr(C)]
#[derive(Debug)]
#[doc = "Register block"]
pub struct RegisterBlock {
    data: Data,
    _reserved1: [u8; 0x0003_fffa],
    dataout: Dataout,
    _reserved2: [u8; 0x02],
    dir: Dir,
}
impl RegisterBlock {
    #[doc = "0x00 - GPIO data register (bit-banded: access pin N via base + (1<<N)<<2)"]
    #[inline(always)]
    pub const fn data(&self) -> &Data {
        &self.data
    }
    #[doc = "0x3fffc - GPIO data register (all 16 pins, bulk read/write)"]
    #[inline(always)]
    pub const fn dataout(&self) -> &Dataout {
        &self.dataout
    }
    #[doc = "0x40000 - GPIO direction register (0=input, 1=output)"]
    #[inline(always)]
    pub const fn dir(&self) -> &Dir {
        &self.dir
    }
}
#[doc = "DATA (rw) register accessor: GPIO data register (bit-banded: access pin N via base + (1<<N)<<2)\n\nYou can [`read`](crate::Reg::read) this register and get [`data::R`]. You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`data::W`]. You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@data`] module"]
#[doc(alias = "DATA")]
pub type Data = crate::Reg<data::DataSpec>;
#[doc = "GPIO data register (bit-banded: access pin N via base + (1<<N)<<2)"]
pub mod data;
#[doc = "DATAOUT (rw) register accessor: GPIO data register (all 16 pins, bulk read/write)\n\nYou can [`read`](crate::Reg::read) this register and get [`dataout::R`]. You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`dataout::W`]. You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@dataout`] module"]
#[doc(alias = "DATAOUT")]
pub type Dataout = crate::Reg<dataout::DataoutSpec>;
#[doc = "GPIO data register (all 16 pins, bulk read/write)"]
pub mod dataout;
#[doc = "DIR (rw) register accessor: GPIO direction register (0=input, 1=output)\n\nYou can [`read`](crate::Reg::read) this register and get [`dir::R`]. You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`dir::W`]. You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@dir`] module"]
#[doc(alias = "DIR")]
pub type Dir = crate::Reg<dir::DirSpec>;
#[doc = "GPIO direction register (0=input, 1=output)"]
pub mod dir;
