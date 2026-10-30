use crate::Message;
use iced::widget::{column, row, text};
use iced::Element;
use modurust_core::dsp::graph::{DspGraph, GraphNodeKind, NodeId};

pub struct ModularGraph {
    graph: DspGraph,
    next_node_id: NodeId,
}

impl ModularGraph {
    pub fn new() -> Self {
        Self {
            graph: DspGraph::new(44100.0),
            next_node_id: 0,
        }
    }

    pub fn add_node(&mut self, kind: GraphNodeKind) -> NodeId {
        let id = self.next_node_id;
        self.graph.add_node_at(id, kind);
        self.next_node_id += 1;
        id
    }

    pub fn remove_node(&mut self, id: NodeId) {
        self.graph.remove_node(id);
    }

    pub fn connect(&mut self, from: NodeId, to: NodeId) {
        self.graph.connect(from, to);
    }

    pub fn disconnect(&mut self, from: NodeId, to: NodeId) {
        self.graph.disconnect(from, to);
    }

    pub fn clear(&mut self) {
        self.graph = DspGraph::new(44100.0);
        self.next_node_id = 0;
    }

    pub fn last_node_id(&self) -> NodeId {
        self.next_node_id.saturating_sub(1)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let osc_button = iced::widget::Button::new(text("Osc"))
            .on_press(Message::GraphAddNode(GraphNodeKind::Oscillator(
                modurust_core::dsp::Waveform::Sine,
                440.0,
            )));
        let filter_button = iced::widget::Button::new(text("Filter"))
            .on_press(Message::GraphAddNode(GraphNodeKind::Filter(
                modurust_core::dsp::FilterMode::LowPass,
                1000.0,
                0.7,
            )));

        column![
            row![osc_button, filter_button].spacing(8),
        ]
        .spacing(8)
        .into()
    }
}