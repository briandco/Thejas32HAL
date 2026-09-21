#[doc = "Register `RBR` reader"]
pub type R = crate::R<RbrSpec>;
#[doc = "Field `DATA` reader - Received data byte"]
pub type DataR = crate::FieldReader;
impl R {
    #[doc = "Bits 0:7 - Received data byte"]
    #[inline(always)]
    pub fn data(&self) -> DataR {
        DataR::new((self.bits & 0xff) as u8)
    }
}
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("RBR").field("data", &self.data()).finish()
    }
}
#[doc = "Receiver Buffer Register\n\nYou can [`read`](crate::Reg::read) this register and get [`rbr::R`](R). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct RbrSpec;
impl crate::RegisterSpec for RbrSpec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`rbr::R`](R) reader structure"]
impl crate::Readable for RbrSpec {}
#[doc = "`reset()` method sets RBR to value 0"]
impl crate::Resettable for RbrSpec {}
