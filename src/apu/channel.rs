pub trait AudioChannel {
    fn tick(&mut self) -> f32;
    fn reset(&mut self, nrx2: u8, nrx3: u8, nrx4: u8);
}
