use game_engine::game::GameWrapper;
use game_engine::loading::DrawTask;
use game_engine::scenes::scene_stack::{SceneStack, SceneStackLoader, SCENE_STACK_FILE_ID, SceneTransition};
use game_engine::graphics::texture::{Texture, TEXTURE_LOAD_ID, TextureLoader};
use game_engine::graphics::transform::{Transform, TRANSFORM_LOAD_ID, TransformLoader};
use game_engine::load::{LOAD_PATH, JSON_FILE, JSONLoad, load_entity_vec, load_deserializable_from_file};
use game_engine::scenes::{SCENES_DIR, SceneLoader, Scene, SceneLoaderJSON};
use std::fmt::{Debug, Formatter};
use game_engine::input::Input;
use game_engine::globals::texture_dict::{TextureDictLoader, TEXTURE_DICT_LOAD_ID};
use game_engine::graphics::render::sprite_renderer::{SpriteRenderer, SpriteRenderError};
use std::ops::{DerefMut};
use anyhow::{Result, Error};
use luminance_glfw::GL33Context;
use luminance_front::context::GraphicsContext;
use luminance_front::pipeline::{PipelineState, PipelineError};
use glam::{Mat4, Vec3};
use specs::{World, WorldExt};
use serde::Deserialize;
use game_engine::components::{ComponentMux, ComponentLoader};
use std::marker::PhantomData;
use luminance_front::texture::Dim2;
use thiserror::Error;
use game_engine::game_loop::GameLoop;
use game_engine::input::multi_input::MultiInput;
use luminance_windowing::{WindowOpt, WindowDim};
use tracing::{debug, trace, info};
use tracing_appender::non_blocking;
use tracing_subscriber::{Registry, EnvFilter};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::layer::SubscriberExt;

fn main() {
    // game_engine::log::init_logger().expect("Logger failed to initialize");
    // tracing_subscriber::fmt::init();
    let app_name = concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION")).to_string();
    let file_appender = tracing_appender::rolling::daily("./", "game_engine.log");
    let (non_blocking_writer, _guard) = non_blocking(file_appender);

    let bunyan_formatting_layer = BunyanFormattingLayer::new(app_name, non_blocking_writer);
    let subscriber = Registry::default()
        .with(EnvFilter::from_default_env())
        .with(JsonStorageLayer)
        .with(bunyan_formatting_layer);

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set global default subscriber");

    info!("TEST INFO");
    trace!("TEST TRACE");
    debug!("TEST DEBUG");

    let game_loop: GameLoop<TestGameWrapper<MultiInput>, MultiInput> = GameLoop::new();
    game_loop.run(
    WindowOpt::default()
        .set_dim(
            WindowDim::Windowed {
                width: 960,
                height: 540
            }
        ),
    "Sprite Render Test".to_string()
    );
}

struct TestGameWrapper<T: Input + Debug> {
    input: PhantomData<T>
}

impl<T: 'static + Input + Debug> TestGameWrapper<T> {
    fn scene_factory(json: JSONLoad) -> Result<Box<dyn SceneLoader<T>>> {
        match json.load_type_id.as_str() {
            SPRITE_RENDER_SCENE_ID => Ok(Box::new(SpriteRenderSceneLoader::new())),
            _ => {Err(Error::msg("Load ID did not match any scene ID"))}
        }
    }
}

impl<T: 'static + Input + Debug> GameWrapper<T> for TestGameWrapper<T> {
    fn register_components(ecs: &mut World) {
        ecs.register::<Texture>();
        ecs.register::<Transform>();
    }

    fn load() -> DrawTask<SceneStack<T>> {
        let ss_loader = SceneStackLoader::new(
            [
                LOAD_PATH,
                SCENES_DIR,
                SCENE_STACK_FILE_ID,
                JSON_FILE
            ].join(""),
            TestGameWrapper::<T>::scene_factory
        );

        let td_loader = TextureDictLoader::new(
            &[
                LOAD_PATH,
                TEXTURE_DICT_LOAD_ID,
                JSON_FILE
            ].join("")
        ).expect("Failed to create TextureDictLoader");

        td_loader.load()
            .map(|texture_dict, (ecs, context)| {
                ecs
                    .write()
                    .expect("Failed to lock World")
                    .insert(texture_dict);

                Ok(())
            })
            .sequence(ss_loader.load(), |(_, scene_stack)| {
                scene_stack
            })
    }
}

pub struct SpriteRenderScene<T: Input + Debug> {
    sprite_renderer: SpriteRenderer,
    phantom_input: PhantomData<T>
}

pub const SPRITE_RENDER_SCENE_ID: &str = "sprite_render_scene";

impl<T: Input + Debug> Debug for SpriteRenderScene<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpriteRenderScene")
            .field("SpriteRenderer", &self.sprite_renderer.render_state)
            .finish()
    }
}

impl<T: Input + Debug> Scene<T> for SpriteRenderScene<T> {
    fn update(&mut self, ecs: &mut World) -> Result<SceneTransition<T>> {
        Ok(SceneTransition::NONE)
    }

    fn draw(&mut self, ecs: &mut World, context: &mut GL33Context) -> Result<()> {
        let back_buffer = context.back_buffer()
            .expect("Failed to get back buffer");

        context.new_pipeline_gate()
            .pipeline::<SpriteRenderError, Dim2, (), (), _>(
                &back_buffer,
                &PipelineState::default().set_clear_color([0.0, 0.0, 0.0, 1.0]),
                |pipeline, mut shading_gate| {
                    self.sprite_renderer.render(
                        &pipeline,
                        &mut shading_gate,
                        &Mat4::orthographic_rh_gl(
                            0.0,
                            960.0,
                            0.0,
                                540.0,
                            -1.0,
                            10.0
                        ),
                        &Mat4::look_at_rh(
                            Vec3::new(0.0, 0.0, 1.0),
                            Vec3::new(0.0, 0.0, 0.0),
                            Vec3::Y
                        ),
                ecs
                    )?;

                    Ok(())
                }
            );
        
        Ok(())
    }

    fn interact(&mut self, ecs: &mut World, input: &T) -> Result<()> {
        Ok(())
    }

    fn get_name(&self) -> String {
        String::from("Sprite Render Test Scene")
    }

    fn is_finished(&self, ecs: &mut World) -> Result<bool> {
        return Ok(false)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct SpriteRenderSceneJSON {}

#[derive(Debug)]
pub struct SpriteRenderSceneLoader<T: Input + Debug> {
    phantom_input: PhantomData<T>
}

impl<T: Input + Debug> SpriteRenderSceneLoader<T> {
    pub fn new() -> Self {
        Self { phantom_input: Default::default() }
    }
}

impl<T: Input + Debug> ComponentMux for SpriteRenderSceneLoader<T> {
    fn map_json_to_loader(json: JSONLoad) -> Result<Box<dyn ComponentLoader>> {
        match json.load_type_id.as_str() {
            TEXTURE_LOAD_ID => Ok(Box::new(TextureLoader::from_json(json)?)),
            TRANSFORM_LOAD_ID => Ok(Box::new(TransformLoader::from_json(json)?)),
            _ => Err(Error::msg("Invalid json load ID"))
        }
    }
}

impl<T: 'static + Input + Debug> SceneLoader<T> for SpriteRenderSceneLoader<T> {
    fn load_scene(&self) -> DrawTask<Box<dyn Scene<T>>> {
        DrawTask::new(|(ecs, context)| {
            let json: SceneLoaderJSON = load_deserializable_from_file(
                &[
                    LOAD_PATH,
                    SCENES_DIR,
                    SPRITE_RENDER_SCENE_ID,
                    JSON_FILE
                ].join(""),
                SPRITE_RENDER_SCENE_ID
            )?;

            let entities = load_entity_vec::<Self>(&json.entity_paths).execute((ecs.clone(), context.clone()))?;

            let sprite_renderer = SpriteRenderer::new(
                context.write().expect("Failed to lock Context").deref_mut()
            );

            Ok(Box::new(SpriteRenderScene {
                sprite_renderer,
                phantom_input: Default::default()
            }) as Box<dyn Scene<T>>)
        })
    }
}