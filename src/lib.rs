use std::collections::{HashMap, HashSet};

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
    callbacks: HashMap<CallbackId, Box<dyn 'a + FnMut(T)>>,
    func: Box<dyn 'a + Fn(&[T]) -> T>,
    cb_id: usize
}

pub struct Reactor<'a, T> {
    inputs: HashMap<CellId, Box<InputCell<T>>>,
    compute: HashMap<CellId, Box<ComputeCell<'a, T>>>,
}

impl<'a, T: Copy + PartialEq> Reactor<'a, T> {
    pub fn new() -> Self {
        Self { inputs: HashMap::new(), compute: HashMap::new() }
    }
    
    pub fn create_input(&mut self, _initial: T) -> InputCellId {
        let id = InputCellId(self.inputs.len());
        self.inputs.insert(CellId::Input(id), Box::new(InputCell::new(_initial,vec![])));
        id
    }
    
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
            cb_id: 1,
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
    
    pub fn value(&self, id: CellId) -> Option<T> {
        match id {
            CellId::Compute(_) => self.compute.get(&id).map(|c| c.val),
            CellId::Input(_) => self.inputs.get(&id).map(|c| c.val)
        }
    }
    
    pub fn set_value(&mut self, _id: InputCellId, _new_value: T) -> bool {
        let _id = CellId::Input(_id);
        if !self.inputs.contains_key(&_id) {
            return false;
        }
        
        if let Some(c) = self.inputs.get_mut(&_id) { 
            c.val = _new_value;
            let mut topo: Vec<CellId> = Vec::new();
            let mut seen: HashSet<CellId> = HashSet::new();
            self.get_topo_order(_id, &mut topo, &mut seen);
            
            topo.reverse();
            self.update_compute_cell_value(&topo[1..]);
        }
        true
    }
    
    fn get_topo_order(&mut self, id: CellId, stack: &mut Vec<CellId>, seen: &mut HashSet<CellId>) {
        seen.insert(id);
        let children = match id {
            CellId::Input(_) => {self.inputs.get_mut(&id).unwrap().children.clone()}
            CellId::Compute(_) => {self.compute.get_mut(&id).unwrap().children.clone()}
        };
        for cid in children {
            if seen.contains(&cid) {
                continue;
            }
            self.get_topo_order(cid, stack, seen);
        }
        stack.push(id);
    }

    fn update_compute_cell_value(&mut self, queue: &[CellId]) {
        for cell_id in queue {
            let parent_values: Vec<T> = self
                .compute
                .get(cell_id)
                .unwrap()
                .parents
                .iter()
                .map(|par| self.value(*par).unwrap())
                .collect();
            
            let cell = self.compute.get_mut(cell_id).unwrap();
            let new_val = (cell.func)(&parent_values);

            if new_val != cell.val {
                cell.val = new_val;
                for cb in cell.callbacks.values_mut() {
                    (cb)(new_val);    
                }
            }
        }
    }
    
    pub fn add_callback<F: 'a + FnMut(T)>(
        &mut self,
        _id: ComputeCellId,
        _callback: F,
    ) -> Option<CallbackId> {
        match self.compute.get_mut(&CellId::Compute(_id)) {
            None => None,
            Some(cell) => {
                let id = CallbackId(cell.cb_id);
                cell.cb_id += 1;
                cell.callbacks.insert(id, Box::new(_callback));
                Some(id)
            }
        }
    }
    
    pub fn remove_callback(
        &mut self,
        cell: ComputeCellId,
        callback: CallbackId,
    ) -> Result<(), RemoveCallbackError> {
        match self.compute.get_mut(&CellId::Compute(cell)) {
            None => Err(RemoveCallbackError::NonexistentCell),
            Some(cell) => {
                match cell.callbacks.remove(&callback) {
                    None => Err(RemoveCallbackError::NonexistentCallback),
                    Some(_) => {
                        cell.callbacks.remove(&callback);
                        Ok(())
                    }
                }
            }
        }
    }
}


