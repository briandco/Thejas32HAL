#[doc = "Register `RESERVED1` reader"]
pub type R = crate::R<Reserved1Spec>;
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.bits())
    }
}
#[doc = "Reserved\n\nYou can [`read`](crate::Reg::read) this register and get [`reserved1::R`](R). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct Reserved1Spec;
impl crate::RegisterSpec for Reserved1Spec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`reserved1::R`](R) reader structure"]
impl crate::Readable for Reserved1Spec {}
#[doc = "`reset()` method sets RESERVED1 to value 0"]
impl crate::Resettable for Reserved1Spec {}
