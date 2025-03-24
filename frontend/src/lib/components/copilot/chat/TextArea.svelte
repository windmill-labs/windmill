<script lang="ts">
    let text = '';
  
    function highlight(text) {
      const escaped = text
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/\n/g, '<br>')
        .replace(/(error)/gi, '<mark class="bg-yellow-300 text-black">$1</mark>');
  
      return escaped;
    }
  </script>
  
  <div class="relative w-full max-w-lg h-48 font-mono">
    <!-- Highlight layer -->
    <div
      class="absolute top-0 left-0 w-full h-full p-2 whitespace-pre-wrap break-words pointer-events-none text-transparent bg-white z-0"
      bind:this={highlightLayer}
      aria-hidden="true"
    >
      {@html highlight(text)}
    </div>
  
    <!-- Textarea on top -->
    <textarea
      bind:value={text}
      on:input
      class="absolute top-0 left-0 w-full h-full p-2 bg-transparent z-10 resize-none focus:outline-none"
      spellcheck="false"
    />
  </div>