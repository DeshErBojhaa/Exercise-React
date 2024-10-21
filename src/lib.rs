use std::collections::HashMap;

/// `InputCellId` is a unique identifier for an input cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputCellId(usize);
/// `ComputeCellId` is a unique identifier for a compute cell.
/// Values of type `InputCellId` and `ComputeCellId` should not be mutually assignable,
/// demonstrated by the following tests:
///
/// ```compile_fail
/// let mut r = react::Reactor::new();
/// let input: react::ComputeCellId = r.create_input(111);
/// ```
///
/// ```compile_fail
/// let mut r = react::Reactor::new();
/// let input = r.create_input(111);
/// let compute: react::InputCellId = r.create_compute(&[react::CellId::Input(input)], |_| 222).unwrap();
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ComputeCellId(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CallbackId(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CellId {
    Input(InputCellId),
    Compute(ComputeCellId),
}

#[derive(Debug, PartialEq, Eq)]
pub enum RemoveCallbackError {
    NonexistentCell,
    NonexistentCallback,
}

pub struct Reactor<T> {
    input: HashMap<CellId, T>,
    compute: HashMap<CellId, Vec<CellId>>,
    func: HashMap<CellId, Box<dyn Fn(&[T])->T>>,
    callbacks: HashMap<CallbackId, Box<dyn FnMut(T)>>,
}

// You are guaranteed that Reactor will only be tested against types that are Copy + PartialEq.
impl<T: Copy + PartialEq + std::fmt::Debug> Reactor<T> {
    pub fn new() -> Self {
        Self{
            input: HashMap::new(),
            compute: HashMap::new(),
            func: HashMap::new(),
            callbacks: HashMap::new(),
        }
    }

    // Creates an input cell with the specified initial value, returning its ID.
    pub fn create_input(&mut self, _initial: T) -> InputCellId {
        let new_id = InputCellId(self.input.len() + 1);
        self.input.insert(CellId::Input(new_id), _initial);
        new_id
    }

    // Creates a compute cell with the specified dependencies and compute function.
    // The compute function is expected to take in its arguments in the same order as specified in
    // `dependencies`.
    // You do not need to reject compute functions that expect more arguments than there are
    // dependencies (how would you check for this, anyway?).
    //
    // If any dependency doesn't exist, returns an Err with that nonexistent dependency.
    // (If multiple dependencies do not exist, exactly which one is returned is not defined and
    // will not be tested)
    //
    // Notice that there is no way to *remove* a cell.
    // This means that you may assume, without checking, that if the dependencies exist at creation
    // time they will continue to exist as long as the Reactor exists.
    pub fn create_compute<F: Fn(&[T]) -> T + 'static>(
        &mut self,
        _dependencies: &[CellId],
        _compute_func: F,
    ) -> Result<ComputeCellId, CellId> {
        for id in _dependencies {
            if !self.input.contains_key(id) && !self.compute.contains_key(id) {
                return Err(*id);
            }
        }
        let id = ComputeCellId(self.compute.len() + 1);
        self.compute.insert(CellId::Compute(id), _dependencies.to_owned());
        self.func.insert(CellId::Compute(id), Box::new(_compute_func));
        Ok(id)
    }

    // Retrieves the current value of the cell, or None if the cell does not exist.
    //
    // You may wonder whether it is possible to implement `get(&self, id: CellId) -> Option<&Cell>`
    // and have a `value(&self)` method on `Cell`.
    //
    // It turns out this introduces a significant amount of extra complexity to this exercise.
    // We chose not to cover this here, since this exercise is probably enough work as-is.
    pub fn value(&self, id: CellId) -> Option<T> {
        match id {
            CellId::Input(id) => {self.input.get(&CellId::Input(id).to_owned()).copied()},
            CellId::Compute(id) => {
                let func = self.func.get(&CellId::Compute(id))?;
                let input_cells = self.compute.get(&CellId::Compute(id))?;
                let args: Vec<T> = input_cells
                    .iter()
                    .map(|cell_id|
                        match cell_id {
                            CellId::Input(_) => self.input.get(cell_id).unwrap().clone(),
                            CellId::Compute(_) => self.value(*cell_id).unwrap().clone(),
                        }
                    )
                    .collect();
                let val = func(args.as_slice());
                Some(val)
            }
        }
    }

    // Sets the value of the specified input cell.
    //
    // Returns false if the cell does not exist.
    //
    // Similarly, you may wonder about `get_mut(&mut self, id: CellId) -> Option<&mut Cell>`, with
    // a `set_value(&mut self, new_value: T)` method on `Cell`.
    //
    // As before, that turned out to add too much extra complexity.
    pub fn set_value(&mut self, _id: InputCellId, _new_value: T) -> bool {
        if !self.input.contains_key(&CellId::Input(_id)) {
            return false;
        }
        self.input.insert(CellId::Input(_id), _new_value).is_some()
    }

    // Adds a callback to the specified compute cell.
    //
    // Returns the ID of the just-added callback, or None if the cell doesn't exist.
    //
    // Callbacks on input cells will not be tested.
    //
    // The semantics of callbacks (as will be tested):
    // For a single set_value call, each compute cell's callbacks should each be called:
    // * Zero times if the compute cell's value did not change as a result of the set_value call.
    // * Exactly once if the compute cell's value changed as a result of the set_value call.
    //   The value passed to the callback should be the final value of the compute cell after the
    //   set_value call.
    pub fn add_callback<F: FnMut(T) + 'static>(
        &mut self,
        _id: ComputeCellId,
        _callback: F,
    ) -> Option<CallbackId> {
        if !self.compute.contains_key(&CellId::Compute(_id)) {
            return None;
        }
        let id = CallbackId(self.callbacks.len() + 1);
        self.callbacks.insert(id, Box::new(_callback));
        Some(id)
    }

    // Removes the specified callback, using an ID returned from add_callback.
    //
    // Returns an Err if either the cell or callback does not exist.
    //
    // A removed callback should no longer be called.
    pub fn remove_callback(
        &mut self,
        cell: ComputeCellId,
        callback: CallbackId,
    ) -> Result<(), RemoveCallbackError> {
        if !self.compute.contains_key(&CellId::Compute(cell)) {
            return Err(RemoveCallbackError::NonexistentCell);
        }

        if !self.callbacks.contains_key(&callback) {
            return Err(RemoveCallbackError::NonexistentCallback);
        }

        self.callbacks.remove(&callback);
        Ok(())
    }
}
