use poise::serenity_prelude as serenity;
use poise::reply::CreateReply;

use crate::{custom_errors::CustomError, Context, Error};

const CHAPTERS: [(&str, &str); 63] = [
    ( "1 - Introduction", "https://www.lua.org/manual/5.2/manual.html#1" ),
    ( "2 - Basic Concepts", "https://www.lua.org/manual/5.2/manual.html#2" ),
    ( "2.1 - Values and Types", "https://www.lua.org/manual/5.2/manual.html#2.1" ),
    ( "2.2 - Environments and the Global Environment", "https://www.lua.org/manual/5.2/manual.html#2.2" ),
    ( "2.3 - Error Handling", "https://www.lua.org/manual/5.2/manual.html#2.3" ),
    ( "2.4 - Metatables and Metamethods", "https://www.lua.org/manual/5.2/manual.html#2.4" ),
    ( "2.5 - Garbage Collection", "https://www.lua.org/manual/5.2/manual.html#2.5" ),
    ( "2.5.1 - Garbage-Collection Metamethods", "https://www.lua.org/manual/5.2/manual.html#2.5.1" ),
    ( "2.5.2 - Weak Tables", "https://www.lua.org/manual/5.2/manual.html#2.5.2" ),
    ( "2.6 - Coroutines", "https://www.lua.org/manual/5.2/manual.html#2.6" ),
    ( "3 - The Language", "https://www.lua.org/manual/5.2/manual.html#3" ),
    ( "3.1 - Lexical Conventions", "https://www.lua.org/manual/5.2/manual.html#3.1" ),
    ( "3.2 - Variables", "https://www.lua.org/manual/5.2/manual.html#3.2" ),
    ( "3.3 - Statements", "https://www.lua.org/manual/5.2/manual.html#3.3" ),
    ( "3.3.1 - Blocks", "https://www.lua.org/manual/5.2/manual.html#3.3.1" ),
    ( "3.3.2 - Chunks", "https://www.lua.org/manual/5.2/manual.html#3.3.2" ),
    ( "3.3.3 - Assignment", "https://www.lua.org/manual/5.2/manual.html#3.3.3" ),
    ( "3.3.4 - Control Structures", "https://www.lua.org/manual/5.2/manual.html#3.3.4" ),
    ( "3.3.5 - For Statement", "https://www.lua.org/manual/5.2/manual.html#3.3.5" ),
    ( "3.3.6 - Function Calls as Statements", "https://www.lua.org/manual/5.2/manual.html#3.3.6" ),
    ( "3.3.7 - Local Declarations", "https://www.lua.org/manual/5.2/manual.html#3.3.7" ),
    ( "3.4 - Expressions", "https://www.lua.org/manual/5.2/manual.html#3.4" ),
    ( "3.4.1 - Arithmetic Operators", "https://www.lua.org/manual/5.2/manual.html#3.4.1" ),
    ( "3.4.2 - Coercion", "https://www.lua.org/manual/5.2/manual.html#3.4.2" ),
    ( "3.4.3 - Relational Operators", "https://www.lua.org/manual/5.2/manual.html#3.4.3" ),
    ( "3.4.4 - Logical Operators", "https://www.lua.org/manual/5.2/manual.html#3.4.4" ),
    ( "3.4.5 - Concatenation", "https://www.lua.org/manual/5.2/manual.html#3.4.5" ),
    ( "3.4.6 - The Length Operator", "https://www.lua.org/manual/5.2/manual.html#3.4.6" ),
    ( "3.4.7 - Precedence", "https://www.lua.org/manual/5.2/manual.html#3.4.7" ),
    ( "3.4.8 - Table Constructors", "https://www.lua.org/manual/5.2/manual.html#3.4.8" ),
    ( "3.4.9 - Function Calls", "https://www.lua.org/manual/5.2/manual.html#3.4.9" ),
    ( "3.4.10 - Function Definitions", "https://www.lua.org/manual/5.2/manual.html#3.4.10" ),
    ( "3.5 - Visibility Rules", "https://www.lua.org/manual/5.2/manual.html#3.5" ),
    ( "4 - The Application Program Interface", "https://www.lua.org/manual/5.2/manual.html#4" ),
    ( "4.1 - The Stack", "https://www.lua.org/manual/5.2/manual.html#4.1" ),
    ( "4.2 - Stack Size", "https://www.lua.org/manual/5.2/manual.html#4.2" ),
    ( "4.3 - Valid and Acceptable Indices", "https://www.lua.org/manual/5.2/manual.html#4.3" ),
    ( "4.4 - C Closures", "https://www.lua.org/manual/5.2/manual.html#4.4" ),
    ( "4.5 - Registry", "https://www.lua.org/manual/5.2/manual.html#4.5" ),
    ( "4.6 - Error Handling in C", "https://www.lua.org/manual/5.2/manual.html#4.6" ),
    ( "4.7 - Handling Yields in C", "https://www.lua.org/manual/5.2/manual.html#4.7" ),
    ( "4.8 - Functions and Types", "https://www.lua.org/manual/5.2/manual.html#4.8" ),
    ( "4.9 - The Debug Interface", "https://www.lua.org/manual/5.2/manual.html#4.9" ),
    ( "5 - The Auxiliary Library", "https://www.lua.org/manual/5.2/manual.html#5" ),
    ( "5.1 - Functions and Types", "https://www.lua.org/manual/5.2/manual.html#5.1" ),
    ( "6 - Standard Libraries", "https://www.lua.org/manual/5.2/manual.html#6" ),
    ( "6.1 - Basic Functions", "https://www.lua.org/manual/5.2/manual.html#6.1" ),
    ( "6.2 - Coroutine Manipulation", "https://www.lua.org/manual/5.2/manual.html#6.2" ),
    ( "6.3 - Modules", "https://www.lua.org/manual/5.2/manual.html#6.3" ),
    ( "6.4 - String Manipulation", "https://www.lua.org/manual/5.2/manual.html#6.4" ),
    ( "6.4.1 - Patterns", "https://www.lua.org/manual/5.2/manual.html#6.4.1" ),
    ( "6.5 - Table Manipulation", "https://www.lua.org/manual/5.2/manual.html#6.5" ),
    ( "6.6 - Mathematical Functions", "https://www.lua.org/manual/5.2/manual.html#6.6" ),
    ( "6.7 - Bitwise Operations", "https://www.lua.org/manual/5.2/manual.html#6.7" ),
    ( "6.8 - Input and Output Facilities", "https://www.lua.org/manual/5.2/manual.html#6.8" ),
    ( "6.9 - Operating System Facilities", "https://www.lua.org/manual/5.2/manual.html#6.9" ),
    ( "6.10 - The Debug Library", "https://www.lua.org/manual/5.2/manual.html#6.10" ),
    ( "7 - Lua Standalone", "https://www.lua.org/manual/5.2/manual.html#7" ),
    ( "8 - Incompatibilities with the Previous Version", "https://www.lua.org/manual/5.2/manual.html#8" ),
    ( "8.1 - Changes in the Language", "https://www.lua.org/manual/5.2/manual.html#8.1" ),
    ( "8.2 - Changes in the Libraries", "https://www.lua.org/manual/5.2/manual.html#8.2" ),
    ( "8.3 - Changes in the API", "https://www.lua.org/manual/5.2/manual.html#8.3" ),
    ( "9 - The Complete Syntax of Lua", "https://www.lua.org/manual/5.2/manual.html#9" ),
];


/// Link chapters in the lua 5.2 manual
#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, track_edits)]
pub async fn lua_chapter (
    ctx: Context<'_>,
    #[description = "Chapter name"]
    #[autocomplete = "autocomplete_chapter"]
    #[rename = "chapter"]
    chapter_name: String,
) -> Result<(), Error> {
    if let Some(chapter) = CHAPTERS.iter().find(|ch| ch.0 == chapter_name){
        let embed = serenity::CreateEmbed::new()
            .title(chapter.0)
            .url(chapter.1)
            .author(serenity::CreateEmbedAuthor::new("Lua 5.2 Reference Manual"))
            .color(serenity::Colour::BLUE);
        let builder = CreateReply::default()
            .embed(embed);
        ctx.send(builder).await?;
    } else {
        return Err(Box::new(CustomError::new(&format!(r#"Could not find chapter "{chapter_name}" in lua manual"#))))
    }
    
    Ok(())
}

#[allow(clippy::unused_async)]
async fn autocomplete_chapter<'a>(
    _ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    CHAPTERS.iter()
        .filter(|ch| {
            let c = ch.0.to_owned();
            c.to_lowercase().contains(&partial.to_lowercase())
        })
        .map(|ch| ch.0.to_owned())
        .collect::<Vec<String>>()
}

const FUNCTIONS: [(&str, &str); 101] = [
    ( "basic", "https://www.lua.org/manual/5.2/manual.html#6.1" ),
    ( "_G", "https://www.lua.org/manual/5.2/manual.html#6.1" ),
    ( "_VERSION", "https://www.lua.org/manual/5.2/manual.html#pdf-_VERSION" ),
    ( "assert", "https://www.lua.org/manual/5.2/manual.html#pdf-assert" ),
    ( "collectgarbage", "https://www.lua.org/manual/5.2/manual.html#pdf-collectgarbage" ),
    ( "error", "https://www.lua.org/manual/5.2/manual.html#pdf-error" ),
    ( "getmetatable", "https://www.lua.org/manual/5.2/manual.html#pdf-getmetatable" ),
    ( "ipairs", "https://www.lua.org/manual/5.2/manual.html#pdf-ipairs" ),
    ( "load", "https://www.lua.org/manual/5.2/manual.html#pdf-load" ),
    ( "next", "https://www.lua.org/manual/5.2/manual.html#pdf-next" ),
    ( "pairs", "https://www.lua.org/manual/5.2/manual.html#pdf-pairs" ),
    ( "pcall", "https://www.lua.org/manual/5.2/manual.html#pdf-pcall" ),
    ( "print", "https://www.lua.org/manual/5.2/manual.html#pdf-print" ),
    ( "rawequal", "https://www.lua.org/manual/5.2/manual.html#pdf-rawequal" ),
    ( "rawget", "https://www.lua.org/manual/5.2/manual.html#pdf-rawget" ),
    ( "rawlen", "https://www.lua.org/manual/5.2/manual.html#pdf-rawlen" ),
    ( "rawset", "https://www.lua.org/manual/5.2/manual.html#pdf-rawset" ),
    ( "require", "https://www.lua.org/manual/5.2/manual.html#pdf-require" ),
    ( "select", "https://www.lua.org/manual/5.2/manual.html#pdf-select" ),
    ( "setmetatable", "https://www.lua.org/manual/5.2/manual.html#pdf-setmetatable" ),
    ( "tonumber", "https://www.lua.org/manual/5.2/manual.html#pdf-tonumber" ),
    ( "tostring", "https://www.lua.org/manual/5.2/manual.html#pdf-tostring" ),
    ( "type", "https://www.lua.org/manual/5.2/manual.html#pdf-type" ),
    ( "xpcall", "https://www.lua.org/manual/5.2/manual.html#pdf-xpcall" ),
    ( "bit32", "https://www.lua.org/manual/5.2/manual.html#6.7" ),
    ( "bit32.arshift", "https://www.lua.org/manual/5.2/manual.html#pdf-bit32.arshift" ),
    ( "bit32.band", "https://www.lua.org/manual/5.2/manual.html#pdf-bit32.band" ),
    ( "bit32.bnot", "https://www.lua.org/manual/5.2/manual.html#pdf-bit32.bnot" ),
    ( "bit32.bor", "https://www.lua.org/manual/5.2/manual.html#pdf-bit32.bor" ),
    ( "bit32.btest", "https://www.lua.org/manual/5.2/manual.html#pdf-bit32.btest" ),
    ( "bit32.bxor", "https://www.lua.org/manual/5.2/manual.html#pdf-bit32.bxor" ),
    ( "bit32.extract", "https://www.lua.org/manual/5.2/manual.html#pdf-bit32.extract" ),
    ( "bit32.lrotate", "https://www.lua.org/manual/5.2/manual.html#pdf-bit32.lrotate" ),
    ( "bit32.lshift", "https://www.lua.org/manual/5.2/manual.html#pdf-bit32.lshift" ),
    ( "bit32.replace", "https://www.lua.org/manual/5.2/manual.html#pdf-bit32.replace" ),
    ( "bit32.rrotate", "https://www.lua.org/manual/5.2/manual.html#pdf-bit32.rrotate" ),
    ( "bit32.rshift", "https://www.lua.org/manual/5.2/manual.html#pdf-bit32.rshift" ),
    ( "debug", "https://www.lua.org/manual/5.2/manual.html#6.10" ),
    ( "debug.getinfo", "https://www.lua.org/manual/5.2/manual.html#pdf-debug.getinfo" ),
    ( "debug.traceback", "https://www.lua.org/manual/5.2/manual.html#pdf-debug.traceback" ),
    ( "math", "https://www.lua.org/manual/5.2/manual.html#6.6" ),
    ( "math.abs", "https://www.lua.org/manual/5.2/manual.html#pdf-math.abs" ),
    ( "math.acos", "https://www.lua.org/manual/5.2/manual.html#pdf-math.acos" ),
    ( "math.asin", "https://www.lua.org/manual/5.2/manual.html#pdf-math.asin" ),
    ( "math.atan", "https://www.lua.org/manual/5.2/manual.html#pdf-math.atan" ),
    ( "math.atan2", "https://www.lua.org/manual/5.2/manual.html#pdf-math.atan2" ),
    ( "math.ceil", "https://www.lua.org/manual/5.2/manual.html#pdf-math.ceil" ),
    ( "math.cos", "https://www.lua.org/manual/5.2/manual.html#pdf-math.cos" ),
    ( "math.cosh", "https://www.lua.org/manual/5.2/manual.html#pdf-math.cosh" ),
    ( "math.deg", "https://www.lua.org/manual/5.2/manual.html#pdf-math.deg" ),
    ( "math.exp", "https://www.lua.org/manual/5.2/manual.html#pdf-math.exp" ),
    ( "math.floor", "https://www.lua.org/manual/5.2/manual.html#pdf-math.floor" ),
    ( "math.fmod", "https://www.lua.org/manual/5.2/manual.html#pdf-math.fmod" ),
    ( "math.frexp", "https://www.lua.org/manual/5.2/manual.html#pdf-math.frexp" ),
    ( "math.huge", "https://www.lua.org/manual/5.2/manual.html#pdf-math.huge" ),
    ( "math.ldexp", "https://www.lua.org/manual/5.2/manual.html#pdf-math.ldexp" ),
    ( "math.log", "https://www.lua.org/manual/5.2/manual.html#pdf-math.log" ),
    ( "math.max", "https://www.lua.org/manual/5.2/manual.html#pdf-math.max" ),
    ( "math.min", "https://www.lua.org/manual/5.2/manual.html#pdf-math.min" ),
    ( "math.modf", "https://www.lua.org/manual/5.2/manual.html#pdf-math.modf" ),
    ( "math.pi", "https://www.lua.org/manual/5.2/manual.html#pdf-math.pi" ),
    ( "math.pow", "https://www.lua.org/manual/5.2/manual.html#pdf-math.pow" ),
    ( "math.rad", "https://www.lua.org/manual/5.2/manual.html#pdf-math.rad" ),
    ( "math.random", "https://www.lua.org/manual/5.2/manual.html#pdf-math.random" ),
    ( "math.randomseed", "https://www.lua.org/manual/5.2/manual.html#pdf-math.randomseed" ),
    ( "math.sin", "https://www.lua.org/manual/5.2/manual.html#pdf-math.sin" ),
    ( "math.sinh", "https://www.lua.org/manual/5.2/manual.html#pdf-math.sinh" ),
    ( "math.sqrt", "https://www.lua.org/manual/5.2/manual.html#pdf-math.sqrt" ),
    ( "math.tan", "https://www.lua.org/manual/5.2/manual.html#pdf-math.tan" ),
    ( "math.tanh", "https://www.lua.org/manual/5.2/manual.html#pdf-math.tanh" ),
    ( "package", "https://www.lua.org/manual/5.2/manual.html#6.3" ),
    ( "package.config", "https://www.lua.org/manual/5.2/manual.html#pdf-package.config" ),
    ( "package.cpath", "https://www.lua.org/manual/5.2/manual.html#pdf-package.cpath" ),
    ( "package.loaded", "https://www.lua.org/manual/5.2/manual.html#pdf-package.loaded" ),
    ( "package.loadlib", "https://www.lua.org/manual/5.2/manual.html#pdf-package.loadlib" ),
    ( "package.path", "https://www.lua.org/manual/5.2/manual.html#pdf-package.path" ),
    ( "package.preload", "https://www.lua.org/manual/5.2/manual.html#pdf-package.preload" ),
    ( "package.searchers", "https://www.lua.org/manual/5.2/manual.html#pdf-package.searchers" ),
    ( "package.searchpath", "https://www.lua.org/manual/5.2/manual.html#pdf-package.searchpath" ),
    ( "string", "https://www.lua.org/manual/5.2/manual.html#6.4" ),
    ( "string.byte", "https://www.lua.org/manual/5.2/manual.html#pdf-string.byte" ),
    ( "string.char", "https://www.lua.org/manual/5.2/manual.html#pdf-string.char" ),
    ( "string.dump", "https://www.lua.org/manual/5.2/manual.html#pdf-string.dump" ),
    ( "string.find", "https://www.lua.org/manual/5.2/manual.html#pdf-string.find" ),
    ( "string.format", "https://www.lua.org/manual/5.2/manual.html#pdf-string.format" ),
    ( "string.gmatch", "https://www.lua.org/manual/5.2/manual.html#pdf-string.gmatch" ),
    ( "string.gsub", "https://www.lua.org/manual/5.2/manual.html#pdf-string.gsub" ),
    ( "string.len", "https://www.lua.org/manual/5.2/manual.html#pdf-string.len" ),
    ( "string.lower", "https://www.lua.org/manual/5.2/manual.html#pdf-string.lower" ),
    ( "string.match", "https://www.lua.org/manual/5.2/manual.html#pdf-string.match" ),
    ( "string.rep", "https://www.lua.org/manual/5.2/manual.html#pdf-string.rep" ),
    ( "string.reverse", "https://www.lua.org/manual/5.2/manual.html#pdf-string.reverse" ),
    ( "string.sub", "https://www.lua.org/manual/5.2/manual.html#pdf-string.sub" ),
    ( "string.upper", "https://www.lua.org/manual/5.2/manual.html#pdf-string.upper" ),
    ( "table", "https://www.lua.org/manual/5.2/manual.html#6.5" ),
    ( "table.concat", "https://www.lua.org/manual/5.2/manual.html#pdf-table.concat" ),
    ( "table.insert", "https://www.lua.org/manual/5.2/manual.html#pdf-table.insert" ),
    ( "table.pack", "https://www.lua.org/manual/5.2/manual.html#pdf-table.pack" ),
    ( "table.remove", "https://www.lua.org/manual/5.2/manual.html#pdf-table.remove" ),
    ( "table.sort", "https://www.lua.org/manual/5.2/manual.html#pdf-table.sort" ),
    ( "table.unpack", "https://www.lua.org/manual/5.2/manual.html#pdf-table.unpack" ),
];

/// Link functions in the lua 5.2 manual
#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, track_edits)]
pub async fn lua_function (
    ctx: Context<'_>,
    #[description = "function name"]
    #[autocomplete = "autocomplete_function"]
    #[rename = "function"]
    function_name: String,
) -> Result<(), Error> {
    if let Some(function) = FUNCTIONS.iter().find(|f| f.0 == function_name){
        let embed = serenity::CreateEmbed::new()
            .title(function.0)
            .url(function.1)
            .author(serenity::CreateEmbedAuthor::new("Lua 5.2 Reference Manual"))
            .color(serenity::Colour::BLUE);
        let builder = CreateReply::default()
            .embed(embed);
        ctx.send(builder).await?;
    } else {
        return Err(Box::new(CustomError::new(&format!(r#"Could not find function "{function_name}" in lua manual"#))))
    }
    Ok(())
}



#[allow(clippy::unused_async)]
async fn autocomplete_function<'a>(
    _ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    FUNCTIONS.iter()
        .filter(|f| {
            let c = f.0.to_owned();
            c.to_lowercase().contains(&partial.to_lowercase())
        })
        .map(|f| f.0.to_owned())
        .collect::<Vec<String>>()
}


