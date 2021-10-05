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
use calendar::date::Date;
use simple_model::model::SimpleModel;
use simple_model::simulation_state_element::SimulationStateElement;
use communication_protocols::error_handling::ErrorHandling;
use communication_protocols::simulation_model::SimulationModel;
use simple_model::simulation_state::{SimulationState, SimulationStateHeader};
use weather::Weather;

use simple_model::infiltration::Infiltration;

pub struct AirFlowModel {
}

impl ErrorHandling for AirFlowModel {
    fn module_name() -> &'static str {
        "Air-flow model"
    }
}

impl SimulationModel for AirFlowModel {
    type Type = Self;

    /// Creates a new ThermalModel from a SimpleModel.    
    fn new(model: &SimpleModel, state: &mut SimulationStateHeader, _n: usize) -> Result<Self, String> {
        
        for (i,space) in model.spaces.iter().enumerate() {
            // Should these initial values be different?
            let initial_vol = 0.0;
            let initial_temp = 0.0;
            let inf_vol_index = state.push(SimulationStateElement::SpaceInfiltrationVolume(i), initial_vol);
            space.set_infiltration_volume_index(inf_vol_index);
            let inf_temp_index = state.push(SimulationStateElement::SpaceInfiltrationTemperature(i), initial_temp);
            space.set_infiltration_temperature_index(inf_temp_index);
        }
        
        
        Ok(AirFlowModel {})
    }

    /// Advances one main_timestep through time. That is,
    /// it performs `self.dt_subdivisions` steps, advancing
    /// `self.dt` seconds in each of them.
    fn march(
        &self,
        date: Date,
        weather: &dyn Weather,
        model: &SimpleModel,
        state: &mut SimulationState,
    ) -> Result<(), String> {
        
        let current_weather = weather.get_weather_data(date);
        let infiltration_temperatature = current_weather.dry_bulb_temperature.expect("Weather does not have dry bulb temperature");
        for space in model.spaces.iter(){
            // Set temperature
            space.set_infiltration_temperature(state, infiltration_temperatature);
            
            // Set volume
            if let Ok(infiltration) = space.infiltration(){
                match infiltration {
                    Infiltration::Constant(v)=>{
                        space.set_infiltration_volume(state, *v);
                    },
                }
            }            
        }

        Ok(())
    }
}
