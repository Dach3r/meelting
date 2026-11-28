module Atoms
  module SubmitButton
    class Component < ViewComponent::Base
      def initialize(**html_options)
        @html_options = html_options
      end

      private
      attr_reader :html_options

      def classes
        class_names(
          "nb-btn flex h-16 w-full bg-lime px-6 text-[17px] text-forest hover:bg-lime-soft",
          html_options[:class]
        )
      end
    end
  end
end
