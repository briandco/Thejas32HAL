#[doc = "Register `EOI` reader"]
pub type R = crate::R<EoiSpec>;
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.bits())
    }
}
#[doc = "Timer end-of-interrupt (read clears interrupt)\n\nYou can [`read`](crate::Reg::read) this register and get [`eoi::R`](R). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct EoiSpec;
impl crate::RegisterSpec for EoiSpec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`eoi::R`](R) reader structure"]
impl crate::Readable for EoiSpec {}
#[doc = "`reset()` method sets EOI to value 0"]
impl crate::Resettable for EoiSpec {}
