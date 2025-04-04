<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { Config } from "./Config";
  import { onMount } from "svelte";

  let input: any = null;
  let inter: any = null;
  let output: any = null;
  let debug: any = "";
  let stat: any = "";

  let config: Config = {
    intermediate_width: 0,
    intermediate_height: 0,
    prioritize_unique: false,
    unique_threshold: 0,
    subpixel_size: 0,
    input_path: null,
    library_path: null,
  };

  async function get_config() {
    config = await invoke("get_config");
  }

  async function set_config() {
    await invoke("set_config", { newConfig: config });
  }

  function image(input: String) {
    // Split the input into two parts: numbers and data after the space
    const [numbers, data] = input.split(" ");

    // Extract width and height using substring
    const width = parseInt(numbers.substring(0, 9), 10); // First 9 digits
    const height = parseInt(numbers.substring(10, 19), 10); // Next 9 digits

    return { width, height, data };
  }

  onMount(async () => {
    await get_config();
  });
</script>

<div class="row">
  <div class="quarter">
    <h6>Input Image</h6>
    <img
      alt="No input selected"
      src="data:image/png;base64,{input ? input.data : ''}"
    />
    {#if input}
    {input.width}x{input.height}
    {/if}
  </div>
  <div class="quarter">
    <h6>Intermediate Image</h6>
    <img
      alt="No input selected"
      src="data:image/png;base64,{inter ? inter.data : ''}"
    />
    {#if inter}
    {inter.width}x{inter.height}
    {/if}
  </div>
  <div class="quarter">
    <h6>Output Image</h6>
    <img
      alt = "No input or library selected"
      src="data:image/png;base64,{output ? output.data : ''}"
    />
    {#if output}
    {output.width}x{output.height}
    {/if}
  </div>
  <div class="quarter col">
    <h6>Configuration</h6>
    <label for="intermediate_width"
      >Intermediate Width: {config.intermediate_width}</label
    >
    <input
      type="range"
      id="intermediate_width"
      min="8"
      max="256"
      bind:value={config.intermediate_width}
      on:change={async () => {
        let ratio = input.height / input.width;
        config.intermediate_height = Math.round(config.intermediate_width * ratio);
        stat = "Setting config..."
        await set_config();
        stat = "Refreshing intermediate image..."
        inter = image(await invoke("reload_image"));
        stat = "Refreshing output image..."
        output = image(await invoke("refresh"));
        stat = "";
      }}
    />

    <div class="separator"></div>

    <label for="intermediate_height"
      >Intermediate Height: {config.intermediate_height}</label
    >
    <input
      type="range"
      id="intermediate_height"
      min="8"
      max="256"
      bind:value={config.intermediate_height}
      on:change={async () => {
        let ratio = input.width / input.height;
        config.intermediate_width = Math.round(config.intermediate_height * ratio);
        stat = "Setting config..."
        await set_config();
        stat = "Refreshing intermediate image..."
        inter = image(await invoke("reload_image"));
        stat = "Refreshing output image..."
        output = image(await invoke("refresh"));
        stat = "";
      }}
    />

    <div class="separator"></div>

    <label for="subpixel_size">Subpixel Size: {config.subpixel_size}</label>
    <input
      type="range"
      id="subpixel_size"
      min="1"
      max="64"
      bind:value={config.subpixel_size}
      on:change={async () => {
        stat = "Setting config..."
        await set_config();
        stat = "Loading library..."
        await invoke("reload_library");
        stat = "Refreshing output image..."
        output = image(await invoke("refresh"));
        stat = "";
      }}
    />

    <div class="separator"></div>

    <label for="prioritize_unique"
      >Prioritize Unique: {config.prioritize_unique}</label
    >
    <input
      type="checkbox"
      id="prioritize_unique"
      bind:checked={config.prioritize_unique}
      on:change={async () => {
        stat = "Setting config..."
        await set_config();
        stat = "Refreshing output image..."
        output = image(await invoke("refresh"));
        stat = "";
      }}
    />

    <div class="separator"></div>

    <label for="unique_threshold"
      >Uniqueness Threshold: {config.unique_threshold}</label
    >
    <input
      type="range"
      id="unique_threshold"
      min="100"
      max="250"
      bind:value={config.unique_threshold}
      on:change={async () => {
        stat = "Setting config..."
        await set_config();
        stat = "Refreshing output image..."
        output = image(await invoke("refresh"));
        stat = "";
      }}
    />

    <div class="separator"></div>

    <div class="row">
      <button
        on:click={async () => {
          stat = "Selecting input image..."
          input = image(await invoke("select_image"));
          await get_config();
          stat = "Refreshing intermediate image..."
          inter = image(await invoke("reload_image"));
          stat = "";
        }}>Select Image</button
      >
      <button
        on:click={async () => {
          stat = "Selecting library..."
          await invoke("select_library");
          await get_config();
          stat = "Loading library..."
          await invoke("reload_library");
          stat = "Refreshing output image..."
          output = image(await invoke("refresh"));
          stat = "";
        }}>Select Library</button
      >
      <button
        on:click={async () => {
          stat = "Exporting image..."
          await invoke("export_image");
          stat = "";
        }}>Export Image</button
      >
    </div>

    <p>{debug}</p>
  </div>
</div>

{#if stat}
  <div class="stat">{stat}</div>
{/if}

<style>

  * {
    font-family: sans-serif;
  }

  input, label, button {
    margin-left: 10px;
  }

  .quarter {
    width: 25%;
  }

  .separator {
    width: 100%;
    height: 20px;
  }

  .fill {
    width: 100%;
    height: 100%;
  }

  .row {
    display: flex;
    flex-direction: row;
  }

  .col {
    display: flex;
    flex-direction: column;
  }

  .stat {
    position: fixed;
    bottom: 0;
    padding: 5px;
    left: 0;
    width: 100%;
    background-color: lightblue;
  }

  img {
    width: 100%;
    height: 100%;
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    image-rendering: pixelated; /* Standard CSS */
    image-rendering: -moz-crisp-edges; /* For Firefox */
    image-rendering: -webkit-optimize-contrast; /* For WebKit browsers */
    -ms-interpolation-mode: nearest-neighbor; /* For older versions of IE */
  }
</style>
