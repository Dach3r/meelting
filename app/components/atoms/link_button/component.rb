module Atoms
  module LinkButton
    class Component < ViewComponent::Base
      BASE = "nb-btn".freeze

      VARIANTS = {
        primary: "nb-btn-raised bg-brand-600 text-white hover:bg-brand-700",
        secondary: "nb-btn-raised bg-white text-ink hover:bg-brand-50",
        signal: "nb-btn-raised-light nb-btn-raised border-white bg-sun-400 text-ink",
        deep: "border-white/40 bg-transparent text-white hover:bg-white/10",
        inverse: "nb-btn-raised bg-white text-brand-700 hover:bg-brand-50",
        inverse_outline: "border-white bg-transparent text-white hover:bg-white/10"
      }.freeze

      SIZES = {
        sm: "px-3.5 py-1.5 text-sm",
        md: "px-4 py-2.5 text-sm",
        lg: "px-6 py-3.5 text-base"
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
