module Atoms
  module Icon
    class Component < ViewComponent::Base
      PATHS = {
        memory: '<path d="m12 3 9 4.5-9 4.5-9-4.5z"/><path d="m3 12 9 4.5 9-4.5"/><path d="m3 16.5 9 4.5 9-4.5"/>',
        shield: '<path d="M12 3 4 6v6c0 4.5 3.4 8 8 9 4.6-1 8-4.5 8-9V6z"/><path d="m9 12 2 2 4-4"/>',
        pulse: '<path d="M3 12h4l3-8 4 16 3-8h4"/>',
        check: '<path d="m5 12.5 4.5 4.5L19 7.5"/>'
      }.freeze

      def initialize(name:, **html_options)
        @name = name
        @html_options = html_options
      end

      def call
        tag.svg(PATHS.fetch(@name).html_safe, viewBox: "0 0 24 24", fill: "none", stroke: "currentColor",
          "stroke-width": 2, "stroke-linecap": "round", "stroke-linejoin": "round", "aria-hidden": true,
          class: "size-5", **@html_options)
      end
    end
  end
end
