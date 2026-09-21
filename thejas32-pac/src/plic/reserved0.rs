#[doc = "Register `RESERVED0` reader"]
pub type R = crate::R<Reserved0Spec>;
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.bits())
    }
}
#[doc = "Reserved\n\nYou can [`read`](crate::Reg::read) this register and get [`reserved0::R`](R). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct Reserved0Spec;
impl crate::RegisterSpec for Reserved0Spec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`reserved0::R`](R) reader structure"]
impl crate::Readable for Reserved0Spec {}
#[doc = "`reset()` method sets RESERVED0 to value 0"]
impl crate::Resettable for Reserved0Spec {}
