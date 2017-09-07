// TODO: (iterator? stream?) adapter that fixes SimpleBlock/Cluster timecodes
use webm::WebmElement;

pub struct TimecodeFixer {
    pub current_offset: u64,
    pub last_cluster_base: u64,
    pub last_observed_timecode: u64,
    pub assumed_duration: u64
}

impl TimecodeFixer {
    pub fn new() -> TimecodeFixer {
        TimecodeFixer {
            current_offset: 0,
            last_cluster_base: 0,
            last_observed_timecode: 0,
            assumed_duration: 33
        }
    }

    pub fn process<'b>(&mut self, element: &WebmElement<'b>) -> WebmElement<'b> {
        match element {
            &WebmElement::Timecode(timecode) => {
                // detect a jump backwards in the source, meaning we need to recalculate our offset
                if timecode < self.last_cluster_base {
                    let next_timecode = self.last_observed_timecode + self.assumed_duration;
                    self.current_offset = next_timecode - timecode;
                }

                // remember the source timecode to detect future jumps
                self.last_cluster_base = timecode;

                // return adjusted timecode
                WebmElement::Timecode(timecode + self.current_offset)
            },
            &WebmElement::SimpleBlock(block) => {
                self.last_observed_timecode = self.last_cluster_base + (block.timecode as u64);
                *element
            },
            _ => *element
        }
    }
}
