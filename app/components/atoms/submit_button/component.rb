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
          "flex h-16 w-full cursor-pointer items-center justify-center rounded-full bg-lime px-6 text-base font-semibold text-forest hover:bg-lime-soft",
          html_options[:class]
        )
      end
    end
  end
end
