use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ComputeCellId(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CallbackId(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputCellId(usize);

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

#[derive(Debug, PartialEq, Eq)]
struct InputCell<T> {
    val: T,
    children: Vec<CellId>,
}

impl<T: Copy> InputCell<T> {
    fn new(val: T, children: Vec<CellId>) -> Self {
        Self { val, children }
    }
}

struct ComputeCell<'a, T> {
    val: T,
    children: Vec<CellId>,
    parents: Vec<CellId>,
    callbacks: HashMap<CallbackId, Box<dyn FnMut(T)>>,
    func: Box<dyn 'a + Fn(&[T]) -> T>,
}

pub struct Reactor<'a, T> {
    inputs: HashMap<CellId, Box<InputCell<T>>>,
    compute: HashMap<CellId, Box<ComputeCell<'a, T>>>,
}

// You are guaranteed that Reactor will only be tested against types that are Copy + PartialEq.
impl<'a, T: Copy + PartialEq> Reactor<'a, T> {
    pub fn new() -> Self {
        Self { inputs: HashMap::new(), compute: HashMap::new() }
    }

    // Creates an input cell with the specified initial value, returning its ID.
    pub fn create_input(&mut self, _initial: T) -> InputCellId {
        let id = InputCellId(self.inputs.len());
        self.inputs.insert(CellId::Input(id), Box::new(InputCell::new(_initial,vec![])));
        id
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
    pub fn create_compute<F: Fn(&[T]) -> T + 'a>(
        &mut self,
        _dependencies: &[CellId],
        _compute_func: F,
    ) -> Result<ComputeCellId, CellId> {
        let mut values: Vec<T> = Vec::new();
        for &d in _dependencies {
            match d {
                CellId::Input(_) => {
                    if !self.inputs.contains_key(&d) {
                        return Err(d);
                    }
                    let cell = self.inputs.get(&d).unwrap();
                    values.push(cell.val);
                }
                CellId::Compute(_) => {
                    if !self.compute.contains_key(&d) {
                        return Err(d);
                    }
                    let cell = self.compute.get(&d).unwrap();
                    values.push(cell.val);
                }
            }
        }
        let id = ComputeCellId(self.compute.len());
        self.compute.insert(CellId::Compute(id), Box::new(ComputeCell {
            val: _compute_func(&values),
            parents: _dependencies.to_vec(),
            children: Vec::new(),
            callbacks: HashMap::new(),
            func: Box::new(_compute_func),
        }));
        for d in _dependencies {
            match d {
                CellId::Input(_) => {
                    let cell = self.inputs.get_mut(&d).unwrap();
                    cell.children.push(CellId::Compute(id));
                }
                CellId::Compute(_) => {
                    let cell = self.compute.get_mut(&d).unwrap();
                    cell.children.push(CellId::Compute(id));
                }
            }
        }
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
            CellId::Compute(_) => self.compute.get(&id).map(|c| c.val),
            CellId::Input(_) => self.inputs.get(&id).map(|c| c.val)
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
        let _id = CellId::Input(_id);
        if !self.inputs.contains_key(&_id) {
            return false;
        }
        let mut children: Vec<CellId> = Vec::new();
        if let Some(c) = self.inputs.get_mut(&_id) { 
            c.val = _new_value;
            for child in c.children.iter() {
                children.push(*child);
            }
        }
        for child in children {
            self.update_compute_cell_value(&child);    
        }
        
        true
    }

    fn update_compute_cell_value(&mut self, id: &CellId) {
        // Collect all parent values BEFORE mutably borrowing `self`
        let parent_values: Vec<T> = self
            .compute
            .get(id)
            .unwrap()
            .parents
            .iter()
            .map(|par| self.value(*par).unwrap())
            .collect();

        // Now it's safe to mutate `self` since we no longer need immutable borrows
        let cell = self.compute.get_mut(id).unwrap();
        let new_val = (cell.func)(&parent_values);

        if new_val != cell.val {
            cell.val = new_val;

            // Call the callbacks with the new value
            for callback in cell.callbacks.values_mut() {
                callback(cell.val);
            }
        }

        // Collect child IDs to avoid borrow issues
        let children: Vec<CellId> = cell.children.clone();
        
        // Now, it's safe to recursively update the children
        for child in children {
            self.update_compute_cell_value(&child);
        }
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
    pub fn add_callback<F: FnMut(T)>(
        &mut self,
        _id: ComputeCellId,
        _callback: F,
    ) -> Option<CallbackId> {
        todo!()
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
        todo!(
            "Remove the callback identified by the CallbackId {callback:?} from the cell {cell:?}"
        )
    }
}


