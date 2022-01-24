//! [fugit]-based timer iplementations of `Delay`, `CountDown`, `Pwm`

use crate::pac::RCC;
use crate::rcc::Clocks;
use crate::timer::General;
pub use crate::timer::{Error, Event, Instance};
use core::convert::TryInto;

pub mod delay;
pub use delay::*;

pub mod counter;
pub use counter::*;
pub mod syscounter;
pub use syscounter::*;

#[cfg(feature = "rtic")]
pub mod monotonic;
#[cfg(feature = "rtic")]
pub use monotonic::*;

pub mod pwm;
pub use pwm::*;

mod hal_02;

/// Timer wrapper
pub struct Timer<TIM, const FREQ: u32> {
    tim: TIM,
}

pub trait TimerExt: Sized {
    /// Non-blocking [Counter] with custom sampling
    fn counter<const FREQ: u32>(self, clocks: &Clocks) -> Counter<Self, FREQ>;
    /// Non-blocking [Counter] with sampling of 1 kHz
    ///
    /// Can wait from 2 ms to 65 sec.
    ///
    /// NOTE: don't use this if your system frequency more than 65 MHz
    fn counter_ms(self, clocks: &Clocks) -> CounterMs<Self> {
        self.counter::<1_000>(clocks)
    }
    /// Non-blocking [Counter] with sampling of 1 MHz
    ///
    /// Can wait from 2 μs to 65 ms.
    fn counter_us(self, clocks: &Clocks) -> CounterUs<Self> {
        self.counter::<1_000_000>(clocks)
    }

    /// Blocking [Delay] with custom sampling
    fn delay<const FREQ: u32>(self, clocks: &Clocks) -> Delay<Self, FREQ>;
    /// Blocking [Delay] with sampling of 1 kHz
    ///
    /// Can wait from 2 ms to 65 sec.
    ///
    /// NOTE: don't use this if your system frequency more than 65 MHz
    fn delay_ms(self, clocks: &Clocks) -> DelayMs<Self> {
        self.delay::<1_000>(clocks)
    }
    /// [Blocking Delay] with sampling of 1 MHz
    ///
    /// Can wait from 2 μs to 65 ms.
    fn delay_us(self, clocks: &Clocks) -> DelayUs<Self> {
        self.delay::<1_000_000>(clocks)
    }
}

impl<TIM: Instance> TimerExt for TIM {
    fn counter<const FREQ: u32>(self, clocks: &Clocks) -> Counter<Self, FREQ> {
        Timer::new(self, clocks).counter()
    }
    fn delay<const FREQ: u32>(self, clocks: &Clocks) -> Delay<Self, FREQ> {
        Timer::new(self, clocks).delay()
    }
}

/// `Timer` with sampling of 1 MHz
pub type TimerUs<TIM> = Timer<TIM, 1_000_000>;

/// `Timer` with sampling of 1 kHz
///
/// NOTE: don't use this if your system frequency more than 65 MHz
pub type TimerMs<TIM> = Timer<TIM, 1_000>;

impl<TIM: Instance, const FREQ: u32> Timer<TIM, FREQ> {
    /// Initialize timer
    pub fn new(tim: TIM, clocks: &Clocks) -> Self {
        unsafe {
            //NOTE(unsafe) this reference will only be used for atomic writes with no side effects
            let rcc = &(*RCC::ptr());
            // Enable and reset the timer peripheral
            TIM::enable(rcc);
            TIM::reset(rcc);
        }

        let mut t = Self { tim };
        t.configure(clocks);
        t
    }

    /// Calculate prescaler depending on `Clocks` state
    pub fn configure(&mut self, clocks: &Clocks) {
        let clk = TIM::timer_clock(clocks);
        assert!(clk.0 % FREQ == 0);
        // Configure timer.  If the u16 conversion panics, try increasing FREQ.
        let psc: u16 = (clk.0 / FREQ - 1).try_into().unwrap();
        self.tim.set_prescaler(psc);
    }

    /// Creates `Counter` that imlements [embedded_hal::timer::CountDown]
    pub fn counter(self) -> Counter<TIM, FREQ> {
        Counter(self)
    }

    /// Creates `Delay` that imlements [embedded_hal::blocking::delay] traits
    pub fn delay(self) -> Delay<TIM, FREQ> {
        Delay(self)
    }

    /// Releases the TIM peripheral
    pub fn release(self) -> TIM {
        self.tim
    }

    /// Starts listening for an `event`
    ///
    /// Note, you will also have to enable the TIM2 interrupt in the NVIC to start
    /// receiving events.
    pub fn listen(&mut self, event: Event) {
        match event {
            Event::Update => {
                // Enable update event interrupt
                self.tim.listen_update_interrupt(true);
            }
        }
    }

    /// Clears interrupt associated with `event`.
    ///
    /// If the interrupt is not cleared, it will immediately retrigger after
    /// the ISR has finished.
    pub fn clear_interrupt(&mut self, event: Event) {
        match event {
            Event::Update => {
                // Clear interrupt flag
                self.tim.clear_update_interrupt_flag();
            }
        }
    }

    /// Stops listening for an `event`
    pub fn unlisten(&mut self, event: Event) {
        match event {
            Event::Update => {
                // Disable update event interrupt
                self.tim.listen_update_interrupt(false);
            }
        }
    }
}
