#[doc = "Register `RESERVED` reader"]
pub type R = crate::R<ReservedSpec>;
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.bits())
    }
}
#[doc = "Reserved\n\nYou can [`read`](crate::Reg::read) this register and get [`reserved::R`](R). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct ReservedSpec;
impl crate::RegisterSpec for ReservedSpec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`reserved::R`](R) reader structure"]
impl crate::Readable for ReservedSpec {}
#[doc = "`reset()` method sets RESERVED to value 0"]
impl crate::Resettable for ReservedSpec {}
