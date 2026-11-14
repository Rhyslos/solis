//! A mesh's attribute layers: adding, reading and setting named values on
//! its points, edges, faces and corners.

use super::{Mesh, VertexId};
use crate::FaceId;
use crate::attributes::{
    AttributeElement, AttributeError, AttributeKind, AttributeValues, Attributes, Domain, HARD,
    Layer, Value,
};

impl Mesh {
    /// Adds an attribute layer called `name`, holding a value of the same
    /// kind as `default` for each element of `domain`, every one `default`
    /// to begin with. Adding a layer that exists with the same domain and
    /// kind changes nothing. Refused if the name is taken by another
    /// domain or kind, or the default isn't a real number.
    pub fn add_attribute(
        &mut self,
        name: &str,
        domain: Domain,
        default: Value,
    ) -> Result<(), AttributeError> {
        self.attributes.add(name, domain, default)
    }

    /// Removes an attribute layer and its values. Returns whether it
    /// existed.
    pub fn remove_attribute(&mut self, name: &str) -> bool {
        self.attributes.remove(name)
    }

    /// Every attribute layer: its name, domain and kind of value.
    pub fn attributes(&self) -> impl Iterator<Item = (&str, Domain, AttributeKind)> + '_ {
        self.attributes
            .iter()
            .map(|(n, l)| (n, l.domain(), l.kind()))
    }

    /// The value of attribute `name` on `element`: `None` if there is no
    /// such layer, the element is of another domain, or isn't in the mesh.
    pub fn attribute(&self, name: &str, element: AttributeElement) -> Option<Value> {
        let layer = self.attributes.get(name)?;
        if layer.domain() != element.domain() || !self.has_element(element) {
            return None;
        }
        Some(match element {
            AttributeElement::Edge(edge) => layer.edge(edge),
            other => layer.at(self.element_index(other)?),
        })
    }

    /// Sets attribute `name` on `element` to `value`. Refused, changing
    /// nothing, for an unknown layer, an element of another domain or not
    /// in the mesh, a value of another kind, or one that isn't a real
    /// number.
    pub fn set_attribute(
        &mut self,
        name: &str,
        element: AttributeElement,
        value: Value,
    ) -> Result<(), AttributeError> {
        let Some(layer) = self.attributes.get(name) else {
            return Err(AttributeError::Unknown(name.to_owned()));
        };
        if layer.domain() != element.domain() {
            return Err(AttributeError::WrongDomain {
                layer: layer.domain(),
                element: element.domain(),
            });
        }
        if layer.kind() != value.kind() {
            return Err(AttributeError::WrongKind {
                layer: layer.kind(),
                value: value.kind(),
            });
        }
        if !value.is_finite() {
            return Err(AttributeError::NotANumber);
        }
        if !self.has_element(element) {
            return Err(AttributeError::NoSuchElement);
        }
        let index = self.element_index(element);
        if let Some(layer) = self.attributes.get_mut(name) {
            match (element, index) {
                (AttributeElement::Edge(edge), _) => layer.set_edge(edge, value),
                (_, Some(i)) => layer.set_at(i, value),
                _ => {}
            }
        }
        if name == HARD {
            self.drop_empty_hard();
        }
        Ok(())
    }

    /// A whole attribute layer: its domain, default and values, for
    /// saving. `None` if there is no layer of that name.
    pub fn attribute_values(&self, name: &str) -> Option<(Domain, Value, AttributeValues)> {
        let layer = self.attributes.get(name)?;
        Some((layer.domain(), layer.default(), layer.values()))
    }

    /// Puts a whole attribute layer in, replacing any of that name, after
    /// checking every value (it may come from a damaged file): values of
    /// the default's kind and real numbers, by-id values for the domain's
    /// elements only (no more than there are point slots, polygon slots or
    /// corners), edge values on edges between two different existing
    /// points. The hard-edge layer ([`HARD`]) is kept with the mesh's hard
    /// edges and can't be put in this way. Refused, changing nothing, if
    /// anything doesn't fit.
    pub fn set_attribute_values(
        &mut self,
        name: &str,
        domain: Domain,
        default: Value,
        values: AttributeValues,
    ) -> Result<(), AttributeError> {
        if name == HARD {
            return Err(AttributeError::NameTaken(name.to_owned()));
        }
        let layer = Layer::from_values(domain, default, values)?;
        let room = match domain {
            Domain::Point => self.positions.len(),
            Domain::Face => self.faces.slot_count(),
            Domain::Corner => self.faces.corners.len(),
            Domain::Edge => usize::MAX,
        };
        if layer.stored() > room {
            return Err(AttributeError::NoSuchElement);
        }
        if layer
            .edges()
            .any(|(e, _)| !self.has_element(AttributeElement::Edge(e)))
        {
            return Err(AttributeError::NoSuchElement);
        }
        self.attributes.put(name, layer);
        Ok(())
    }

    /// Whether `element` is in the mesh. An edge counts if its two points
    /// exist and differ (it needn't have a polygon, as with hard edges).
    pub fn has_element(&self, element: AttributeElement) -> bool {
        match element {
            AttributeElement::Point(v) => self.position(v).is_some(),
            AttributeElement::Edge(edge) => {
                let (a, b) = edge.ends();
                a != b && self.position(a).is_some() && self.position(b).is_some()
            }
            AttributeElement::Face(f) => self.face(f).is_some(),
            AttributeElement::Corner(f, i) => self.face(f).is_some_and(|c| i < c.len()),
        }
    }

    /// Where `element`'s values are kept in a dense layer: its id, or for
    /// a corner its place in the mesh's list of corners.
    fn element_index(&self, element: AttributeElement) -> Option<usize> {
        match element {
            AttributeElement::Point(v) => Some(v.0 as usize),
            AttributeElement::Face(f) => Some(f.0 as usize),
            AttributeElement::Corner(f, i) => {
                Some(*self.faces.starts.get(f.0 as usize)? as usize + i)
            }
            AttributeElement::Edge(_) => None,
        }
    }

    /// Where corner `v` of polygon `face` is kept, if `face` has it.
    pub(crate) fn corner_index(&self, face: FaceId, v: VertexId) -> Option<usize> {
        let i = self.face(face)?.iter().position(|c| *c == v)?;
        Some(self.faces.starts[face.0 as usize] as usize + i)
    }

    /// The attribute layers, for the builder.
    pub(crate) fn attribute_layers(&self) -> &Attributes {
        &self.attributes
    }

    /// The attribute layers, for the builder, to change.
    pub(crate) fn attribute_layers_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}
