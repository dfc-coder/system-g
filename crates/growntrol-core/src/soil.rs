#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoilCalibration {
    pub dry_raw: u16,
    pub wet_raw: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoilError {
    EmptySamples,
    InvalidCalibration,
}

impl SoilCalibration {
    pub fn validate(self) -> Result<(), SoilError> {
        if self.dry_raw == self.wet_raw {
            return Err(SoilError::InvalidCalibration);
        }
        Ok(())
    }

    pub fn percentage(self, raw: u16) -> Result<u8, SoilError> {
        self.validate()?;

        let dry = i32::from(self.dry_raw);
        let wet = i32::from(self.wet_raw);
        let value = i32::from(raw);
        let span = wet - dry;
        let percent = ((value - dry) * 100) / span;

        Ok(percent.clamp(0, 100) as u8)
    }
}

pub fn median_sample(samples: &mut [u16]) -> Result<u16, SoilError> {
    if samples.is_empty() {
        return Err(SoilError::EmptySamples);
    }

    samples.sort_unstable();
    let middle = samples.len() / 2;

    if samples.len() % 2 == 1 {
        Ok(samples[middle])
    } else {
        let lower = u32::from(samples[middle - 1]);
        let upper = u32::from(samples[middle]);
        Ok(((lower + upper) / 2) as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_rejects_isolated_adc_noise() {
        let mut samples = [2010, 2000, 3990, 1995, 2005, 2015, 1990];
        assert_eq!(median_sample(&mut samples), Ok(2005));
    }

    #[test]
    fn calibration_maps_dry_and_wet_endpoints() {
        let calibration = SoilCalibration {
            dry_raw: 3200,
            wet_raw: 1400,
        };

        assert_eq!(calibration.percentage(3200), Ok(0));
        assert_eq!(calibration.percentage(1400), Ok(100));
        assert_eq!(calibration.percentage(2300), Ok(50));
    }

    #[test]
    fn calibration_clamps_values_outside_range() {
        let calibration = SoilCalibration {
            dry_raw: 3200,
            wet_raw: 1400,
        };

        assert_eq!(calibration.percentage(4000), Ok(0));
        assert_eq!(calibration.percentage(800), Ok(100));
    }
}
