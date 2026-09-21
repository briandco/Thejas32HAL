#[doc = "Register `SR0` reader"]
pub type R = crate::R<Sr0Spec>;
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.bits())
    }
}
#[doc = "Status Register 0\n\nYou can [`read`](crate::Reg::read) this register and get [`sr0::R`](R). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct Sr0Spec;
impl crate::RegisterSpec for Sr0Spec {
    type Ux = u8;
}
#[doc = "`read()` method returns [`sr0::R`](R) reader structure"]
impl crate::Readable for Sr0Spec {}
#[doc = "`reset()` method sets SR0 to value 0xd8"]
impl crate::Resettable for Sr0Spec {
    const RESET_VALUE: u8 = 0xd8;
}
