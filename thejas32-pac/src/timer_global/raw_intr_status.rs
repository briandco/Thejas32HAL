#[doc = "Register `RAW_INTR_STATUS` reader"]
pub type R = crate::R<RawIntrStatusSpec>;
#[doc = "Field `TIMER0` reader - Timer 0 raw interrupt status"]
pub type Timer0R = crate::BitReader;
#[doc = "Field `TIMER1` reader - Timer 1 raw interrupt status"]
pub type Timer1R = crate::BitReader;
#[doc = "Field `TIMER2` reader - Timer 2 raw interrupt status"]
pub type Timer2R = crate::BitReader;
impl R {
    #[doc = "Bit 0 - Timer 0 raw interrupt status"]
    #[inline(always)]
    pub fn timer0(&self) -> Timer0R {
        Timer0R::new((self.bits & 1) != 0)
    }
    #[doc = "Bit 1 - Timer 1 raw interrupt status"]
    #[inline(always)]
    pub fn timer1(&self) -> Timer1R {
        Timer1R::new(((self.bits >> 1) & 1) != 0)
    }
    #[doc = "Bit 2 - Timer 2 raw interrupt status"]
    #[inline(always)]
    pub fn timer2(&self) -> Timer2R {
        Timer2R::new(((self.bits >> 2) & 1) != 0)
    }
}
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("RAW_INTR_STATUS")
            .field("timer0", &self.timer0())
            .field("timer1", &self.timer1())
            .field("timer2", &self.timer2())
            .finish()
    }
}
#[doc = "Global raw timer interrupt status register\n\nYou can [`read`](crate::Reg::read) this register and get [`raw_intr_status::R`](R). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct RawIntrStatusSpec;
impl crate::RegisterSpec for RawIntrStatusSpec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`raw_intr_status::R`](R) reader structure"]
impl crate::Readable for RawIntrStatusSpec {}
#[doc = "`reset()` method sets RAW_INTR_STATUS to value 0"]
impl crate::Resettable for RawIntrStatusSpec {}
