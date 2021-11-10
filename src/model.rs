/*
MIT License
Copyright (c) 2021 Germán Molina
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:
The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.
THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

// use crate::Float;
use calendar::Date;
use communication_protocols::error_handling::ErrorHandling;
use communication_protocols::simulation_model::SimulationModel;
use simple_model::{
    Infiltration, SimpleModel, SimulationState, SimulationStateElement, SimulationStateHeader,
};
use weather::{CurrentWeather, Weather};

use crate::resolvers::*;

pub type Resolver = Box<dyn Fn(&CurrentWeather, &mut SimulationState)>;

pub struct AirFlowModel {
    infiltration_calcs: Vec<Resolver>,
}

impl ErrorHandling for AirFlowModel {
    fn module_name() -> &'static str {
        "Air-flow model"
    }
}

impl SimulationModel for AirFlowModel {
    type Type = Self;

    /// Creates a new ThermalModel from a SimpleModel.    
    fn new(
        model: &SimpleModel,
        state: &mut SimulationStateHeader,
        _n: usize,
    ) -> Result<Self, String> {
        let mut infiltration_calcs = Vec::with_capacity(model.spaces.len());

        for (i, space) in model.spaces.iter().enumerate() {
            // Should these initial values be different?
            let initial_vol = 0.0;
            let initial_temp = 0.0;
            let inf_vol_index = state.push(
                SimulationStateElement::SpaceInfiltrationVolume(i),
                initial_vol,
            );
            space.set_infiltration_volume_index(inf_vol_index);
            let inf_temp_index = state.push(
                SimulationStateElement::SpaceInfiltrationTemperature(i),
                initial_temp,
            );
            space.set_infiltration_temperature_index(inf_temp_index);

            // Pre-process infiltration calculations
            if let Ok(infiltration) = space.infiltration() {
                let infiltration_fn = match infiltration {
                    Infiltration::Constant(v) => constant_resolver(space, *v)?,
                    Infiltration::Blast(v) => blast_resolver(space, *v)?,
                    Infiltration::Doe2(v) => doe2_resolver(space, *v)?,
                    Infiltration::DesignFlowRate(a, b, c, d, v) => {
                        design_flow_rate_resolver(space, *a, *b, *c, *d, *v)?
                    }
                    Infiltration::EffectiveAirLeakageArea(al) => {
                        effective_air_leakage_resolver(space, *al)?
                    }
                };
                infiltration_calcs.push(infiltration_fn);
            } else {
                // Does nothing
                infiltration_calcs.push(Box::new(
                    move |_current_weather: &CurrentWeather, _state: &mut SimulationState| {},
                ));
            }
        }

        Ok(AirFlowModel { infiltration_calcs })
    }

    /// Advances one main_timestep through time. That is,
    /// it performs `self.dt_subdivisions` steps, advancing
    /// `self.dt` seconds in each of them.
    fn march(
        &self,
        date: Date,
        weather: &dyn Weather,
        _model: &SimpleModel,
        state: &mut SimulationState,
    ) -> Result<(), String> {
        // Process infiltration
        let current_weather = weather.get_weather_data(date);
        for func in self.infiltration_calcs.iter() {
            func(&current_weather, state)
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schedule::ScheduleConstant;
    use simple_model::Space;
    use weather::SyntheticWeather;

    #[test]
    fn test_infiltration() {
        let mut simple_model = SimpleModel::new("model".to_string());
        let mut state_header = SimulationStateHeader::new();

        let mut space = Space::new("some space".to_string());
        space.set_infiltration(Infiltration::Doe2(1.));
        let i = state_header.push(SimulationStateElement::SpaceDryBulbTemperature(0), 22.);
        space.set_dry_bulb_temperature_index(i);
        let space = simple_model.add_space(space);

        let model = AirFlowModel::new(&simple_model, &mut state_header, 1)
            .expect("Could not build AirFlow model");
        let mut state = state_header
            .take_values()
            .expect("Could not take values form SimualationStateHeader");

        /*
        This test is essentially the same as in test_design_blast_flow_rate().

        "At a winter condition of 40◦C deltaT and 6 m/s
        (13.4 mph) windspeed, these coefficients would increase the infiltration
        rate by a factor of 2.75."
        */
        let space_temp = space
            .dry_bulb_temperature(&state)
            .expect("Could not get Dry BUlb Temp from space");
        let mut weather = SyntheticWeather::new();
        weather.dry_bulb_temperature = Box::new(ScheduleConstant::new(space_temp - 40.));
        weather.wind_speed = Box::new(ScheduleConstant::new(6.));

        let date = Date {
            month: 1,
            day: 1,
            hour: 10.,
        };

        // It should be initialized as Zero
        let inf = space.infiltration_volume(&state).unwrap();
        assert!(inf < 1e-9);

        model
            .march(date, &weather, &simple_model, &mut state)
            .unwrap();

        // Check values.
        let inf = space.infiltration_volume(&state).unwrap();
        assert!((1.34 - inf).abs() < 0.02);

        // ... A windspeed of 4.47 m/s (10 mph) gives a factor of 1.0.
        let mut weather = SyntheticWeather::new();
        weather.dry_bulb_temperature = Box::new(ScheduleConstant::new(space_temp - 40.));
        weather.wind_speed = Box::new(ScheduleConstant::new(4.47));
        model
            .march(date, &weather, &simple_model, &mut state)
            .unwrap();

        // Check values.
        let inf = space.infiltration_volume(&state).unwrap();
        assert!((1. - inf).abs() < 0.02);
    }
}
