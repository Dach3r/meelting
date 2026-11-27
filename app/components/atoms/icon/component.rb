module Atoms
  module Icon
    class Component < ViewComponent::Base
      def initialize(name:, **html_options)
        @name = name.to_s.dasherize
        @html_options = html_options
      end

      def call
        helpers.lucide_icon(@name, "aria-hidden" => true, "class" => "size-5", **@html_options.stringify_keys)
      end
    end
  end
end
