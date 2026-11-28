module Atoms
  module LinkButton
    class Component < ViewComponent::Base
      BASE = "nb-btn".freeze

      VARIANTS = {
        primary: "bg-forest text-lime hover:bg-forest-active",
        secondary: "border-ink bg-white text-ink hover:bg-cream",
        accent: "bg-lime text-forest"
      }.freeze

      SIZES = {
        sm: "h-10 px-4 text-sm",
        md: "h-11 px-5 text-[15px]",
        lg: "h-14 px-6 text-[17px]",
        block: "h-14 w-full px-6 text-[17px]"
      }.freeze

      def initialize(href:, variant: :primary, size: :md, **html_options)
        @href = href
        @variant = variant
        @size = size
        @extra_class = html_options.delete(:class)
        @html_options = html_options
      end

      def call
        link_to content, @href, class: class_names(BASE, VARIANTS.fetch(@variant), SIZES.fetch(@size), @extra_class), **@html_options
      end
    end
  end
end
