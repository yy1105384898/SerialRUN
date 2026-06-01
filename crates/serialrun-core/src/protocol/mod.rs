pub mod can;
pub mod custom;
pub mod flasher;
pub mod i2c_spi;
pub mod iec104;
pub mod modbus;
pub mod modbus_bridge;
pub mod modbus_master;
pub mod modbus_simulator;
pub mod modbus_tcp;
pub mod serial_scope;

pub use can::{CanAnalyzer, CanFilter, CanFrame, CanIdStats};
pub use custom::{ProtocolFrame, ProtocolParser};
pub use flasher::FlashResult;
pub use i2c_spi::{I2cResult, I2cScanEntry, SpiResult};
pub use iec104::{
    build_startdt_act, build_stopdt_act, build_testfr_act, Iec104Apdu, Iec104FrameKind,
};
pub use modbus::{ModbusFrame, ModbusFunction, ModbusParser};
pub use modbus_bridge::{start_bridge, BridgeConfig, BridgeLogEntry};
pub use modbus_master::{ModbusMaster, ModbusMasterError, ModbusMasterResult};
pub use modbus_simulator::{
    start_simulator, update_coil, update_holding_register, SimulatorConfig, SimulatorLogEntry,
    SimulatorMode, SimulatorState,
};
pub use modbus_tcp::ModbusTcpFrame;
pub use serial_scope::{ScopeConfig, ScopeStats, SerialScope};
