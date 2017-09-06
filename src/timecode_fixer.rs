// TODO: (iterator? stream?) adapter that fixes SimpleBlock/Cluster timecodes
use webm::WebmElement;

pub struct TimecodeFixer {
}

impl TimecodeFixer {
    pub fn new() -> TimecodeFixer {
        TimecodeFixer {
        }
    }

    pub fn process<'b>(&mut self, element: &WebmElement<'b>) -> WebmElement<'b> {
        match element {
            _ => *element
        }
    }
}
