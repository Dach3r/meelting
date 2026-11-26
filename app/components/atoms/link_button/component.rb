module Atoms
  module LinkButton
    class Component < ViewComponent::Base
      BASE = "inline-flex items-center justify-center gap-2 rounded-ui border font-medium whitespace-nowrap transition-colors".freeze

      VARIANTS = {
        primary: "border-accent bg-accent text-white hover:bg-accent/90",
        secondary: "border-line bg-surface text-ink hover:bg-canvas",
        signal: "border-signal bg-signal text-signal-ink hover:bg-signal/90",
        deep: "border-deep-line bg-deep-surface text-deep-ink hover:bg-deep-line",
        inverse: "border-white bg-white text-accent hover:bg-white/90",
        inverse_outline: "border-white/50 bg-transparent text-white hover:bg-white/10"
      }.freeze

      SIZES = {
        sm: "px-3 py-1.5 text-sm",
        md: "px-3.5 py-2 text-sm",
        lg: "px-5 py-2.5 text-sm"
      }.freeze

      def initialize(href:, variant: :primary, size: :md, **html_options)
        @href = href
        @variant = variant
        @size = size
        @html_options = html_options
      end

      def call
        link_to content, @href, class: class_names(BASE, VARIANTS.fetch(@variant), SIZES.fetch(@size)), **@html_options
      end
    end
  end
end
