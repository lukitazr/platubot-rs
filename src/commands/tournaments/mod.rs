mod capturas;
mod crear;
mod finalizar;
mod fixture;
mod graphics;
mod inscribir;
mod programar;
mod resultado;
mod shared;
mod sortear;
mod tabla;
mod torneo;
mod ver;
mod wo;

use crate::database::repositories::tournaments as repo;

use self::capturas::capturas;
use self::crear::crear;
use self::finalizar::finalizar;
use self::fixture::fixture;
use self::inscribir::inscribir;
use self::programar::programar;
use self::resultado::resultado;
use self::sortear::sortear;
use self::tabla::tabla;
use self::ver::ver;
use self::wo::wo;
