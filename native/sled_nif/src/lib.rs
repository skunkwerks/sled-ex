#![warn(clippy::all)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]
use std::path::PathBuf;

use rustler::{
    resource_struct_init, Encoder, Env, Error, NifStruct, OwnedBinary, ResourceArc, Term,
};

use sled::SegmentMode;

mod atoms {
    rustler::rustler_atoms! {
        atom ok;
        atom error;
        atom nil;
        atom linear;
        atom gc;
    }
}

struct SledConfig {
    pub config: sled::Config,
}

struct SledDb {
    pub db: sled::Db,
}

#[derive(NifStruct)]
#[module = "Sled.Config.Options"]
struct SledConfigOptions {
    pub path: Option<String>,
    pub flush_every_ms: Option<(bool, Option<u64>)>,
    pub temporary: Option<bool>,
    pub create_new: Option<bool>,
    pub cache_capacity: Option<u64>,
    pub print_profile_on_drop: Option<bool>,
    pub use_compression: Option<bool>,
    pub compression_factor: Option<i32>,
    pub snapshot_after_ops: Option<u64>,
    pub segment_cleanup_threshold: Option<u8>,
    pub segment_cleanup_skew: Option<usize>,
    pub segment_mode: rustler::Atom,
    pub snapshot_path: Option<(bool, Option<String>)>,
    pub idgen_persist_interval: Option<u64>,
    pub read_only: Option<bool>,
}

rustler::rustler_export_nifs! {
    "Elixir.Sled.Native",
    [
        ("sled_config_new", 1, sled_config_new),
        ("sled_config_open", 1, sled_config_open),
        ("sled_config_inspect", 1, sled_config_inspect),
        ("sled_open", 1, sled_open),
        ("sled_insert", 3, sled_insert),
        ("sled_get", 2, sled_get)
    ],
    Some(on_load)
}

fn on_load(env: Env, _info: Term) -> bool {
    resource_struct_init!(SledConfig, env);
    resource_struct_init!(SledDb, env);
    true
}

fn sled_config_new<'a>(env: Env<'a>, args: &[Term<'a>]) -> Result<Term<'a>, Error> {
    let config_options: SledConfigOptions = args[0].decode()?;
    let mut config = sled::Config::new();
    config = set_if_configured(
        config,
        sled::Config::flush_every_ms,
        flush_every_ms_to_rust(config_options.flush_every_ms)?,
    );
    config = set_if_configured(
        config,
        sled::Config::segment_mode,
        segment_mode_to_rust(config_options.segment_mode)?,
    );
    config = set_if_configured(
        config,
        sled::Config::snapshot_path,
        snapshot_path_to_rust(config_options.snapshot_path)?,
    );
    config = set_if_configured(config, sled::Config::path, config_options.path);
    config = set_if_configured(config, sled::Config::temporary, config_options.temporary);
    config = set_if_configured(config, sled::Config::create_new, config_options.create_new);
    config = set_if_configured(config, sled::Config::read_only, config_options.read_only);
    config = set_if_configured(
        config,
        sled::Config::cache_capacity,
        config_options.cache_capacity,
    );
    config = set_if_configured(
        config,
        sled::Config::print_profile_on_drop,
        config_options.print_profile_on_drop,
    );
    config = set_if_configured(
        config,
        sled::Config::use_compression,
        config_options.use_compression,
    );
    config = set_if_configured(
        config,
        sled::Config::compression_factor,
        config_options.compression_factor,
    );
    config = set_if_configured(
        config,
        sled::Config::snapshot_after_ops,
        config_options.snapshot_after_ops,
    );
    config = set_if_configured(
        config,
        sled::Config::segment_cleanup_threshold,
        config_options.segment_cleanup_threshold,
    );
    config = set_if_configured(
        config,
        sled::Config::segment_cleanup_skew,
        config_options.segment_cleanup_skew,
    );
    config = set_if_configured(
        config,
        sled::Config::idgen_persist_interval,
        config_options.idgen_persist_interval,
    );

    Ok(ResourceArc::new(SledConfig { config }).encode(env))
}

fn set_if_configured<T>(
    config: sled::Config,
    setter: fn(sled::Config, T) -> sled::Config,
    value: Option<T>,
) -> sled::Config {
    match value {
        Some(value) => setter(config, value),
        None => config,
    }
}

#[allow(clippy::option_option)]
fn flush_every_ms_to_rust(
    value: Option<(bool, Option<u64>)>,
) -> Result<Option<Option<u64>>, Error> {
    match value {
        Some((true, Some(ms))) => Ok(Some(Some(ms))),
        Some((false, None)) => Ok(Some(None)),
        Some((true, None)) => Err(Error::BadArg),
        Some((false, _)) => Err(Error::BadArg),
        None => Ok(None),
    }
}

fn segment_mode_to_rust(atom: rustler::types::Atom) -> Result<Option<SegmentMode>, Error> {
    match atom {
        atom if atom == atoms::linear() => Ok(Some(SegmentMode::Linear)),
        atom if atom == atoms::gc() => Ok(Some(SegmentMode::Gc)),
        atom if atom == atoms::nil() => Ok(None),
        _ => Err(Error::BadArg),
    }
}

#[allow(clippy::option_option)]
fn snapshot_path_to_rust(
    value: Option<(bool, Option<String>)>,
) -> Result<Option<Option<PathBuf>>, Error> {
    match value {
        Some((true, Some(snapshot_path))) => Ok(Some(Some(PathBuf::from(snapshot_path)))),
        Some((false, None)) => Ok(Some(None)),
        Some((true, None)) => Err(Error::BadArg),
        Some((false, _)) => Err(Error::BadArg),
        None => Ok(None),
    }
}

fn sled_config_open<'a>(env: Env<'a>, args: &[Term<'a>]) -> Result<Term<'a>, Error> {
    let config: ResourceArc<SledConfig> = args[0].decode()?;
    do_sled_open(config.config.open(), env)
}

fn sled_open<'a>(env: Env<'a>, args: &[Term<'a>]) -> Result<Term<'a>, Error> {
    let db_name: String = args[0].decode()?;
    do_sled_open(sled::open(db_name), env)
}

fn do_sled_open<'a>(result: sled::Result<sled::Db>, env: Env<'a>) -> Result<Term<'a>, Error> {
    match result {
        Ok(db) => {
            let resource = ResourceArc::new(SledDb { db });
            Ok((atoms::ok(), resource).encode(env))
        }
        Err(_) => Ok(atoms::error().encode(env)),
    }
}

fn sled_config_inspect<'a>(env: Env<'a>, args: &[Term<'a>]) -> Result<Term<'a>, Error> {
    let config: ResourceArc<SledConfig> = args[0].decode()?;
    Ok(format!("{:?}", config.config).encode(env))
}

fn sled_insert<'a>(env: Env<'a>, args: &[Term<'a>]) -> Result<Term<'a>, Error> {
    let resource: ResourceArc<SledDb> = args[0].decode()?;
    let k: String = args[1].decode()?;
    let v: String = args[2].decode()?;
    resource.db.insert(k.as_bytes(), v.as_bytes()).unwrap();

    Ok(atoms::ok().encode(env))
}

fn sled_get<'a>(env: Env<'a>, args: &[Term<'a>]) -> Result<Term<'a>, Error> {
    let resource: ResourceArc<SledDb> = args[0].decode()?;
    let k: String = args[1].decode()?;
    match resource.db.get(k.as_bytes()) {
        Ok(Some(v)) => Ok((atoms::ok(), SledIVec(v)).encode(env)),
        Ok(None) => Ok((atoms::ok(), atoms::nil()).encode(env)),
        Err(_inner) => Ok(atoms::error().encode(env)),
    }
}

struct SledIVec(sled::IVec);

impl Encoder for SledIVec {
    fn encode<'a>(&self, env: Env<'a>) -> Term<'a> {
        let len = self.0.len();
        let mut bin = OwnedBinary::new(len).unwrap();
        bin.as_mut_slice().copy_from_slice(self.0.as_ref());
        bin.release(env).to_term(env)
    }
}
