#[doc = "Register `INTR_STATUS` reader"]
pub type R = crate::R<IntrStatusSpec>;
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.bits())
    }
}
#[doc = "Timer interrupt status\n\nYou can [`read`](crate::Reg::read) this register and get [`intr_status::R`](R). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct IntrStatusSpec;
impl crate::RegisterSpec for IntrStatusSpec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`intr_status::R`](R) reader structure"]
impl crate::Readable for IntrStatusSpec {}
#[doc = "`reset()` method sets INTR_STATUS to value 0"]
impl crate::Resettable for IntrStatusSpec {}
