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
          "flex h-14 w-full cursor-pointer items-center justify-center rounded-xl bg-forest px-6 font-display text-[17px] font-bold text-lime hover:bg-forest-active",
          html_options[:class]
        )
      end
    end
  end
end
