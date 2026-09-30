Feature: Finger command line
  As the operator of thimbl
  I want an explicit command surface
  So that serving is deliberate and setup never binds a socket

  Scenario: Bare thimbl prints usage and does not listen
    When thimbl runs with no subcommand
    Then it exits non-zero
    And the output names the serve command

  Scenario: The help flag prints usage and exits zero
    When thimbl runs with "--help"
    Then it exits with code 0
    And the output names the serve command
    And the output names the init command

  Scenario: The serve command accepts the help flag
    When thimbl runs with "serve --help"
    Then it exits with code 0
    And the output names the serve command

  Scenario: The init command accepts the help flag
    When thimbl runs with "init --help"
    Then it exits with code 0
    And the output names the init command

  Scenario: An invalid port argument is refused
    When thimbl runs with "serve --port nope"
    Then it exits with code 2
    And the output names the serve command

  Scenario: An unknown subcommand is refused
    When thimbl runs with "frobnicate"
    Then it exits with code 2
    And the output names the serve command

  Scenario: An unknown init argument is refused
    When thimbl runs with "init --bogus"
    Then it exits with code 2
    And the output names the init command
