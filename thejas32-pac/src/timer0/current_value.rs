#[doc = "Register `CURRENT_VALUE` reader"]
pub type R = crate::R<CurrentValueSpec>;
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.bits())
    }
}
#[doc = "Timer current value\n\nYou can [`read`](crate::Reg::read) this register and get [`current_value::R`](R). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct CurrentValueSpec;
impl crate::RegisterSpec for CurrentValueSpec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`current_value::R`](R) reader structure"]
impl crate::Readable for CurrentValueSpec {}
#[doc = "`reset()` method sets CURRENT_VALUE to value 0"]
impl crate::Resettable for CurrentValueSpec {}
