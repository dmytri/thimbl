Feature: Server lifecycle
  As the single user of the finger server
  I want the server to stop cleanly on demand
  So that its shutdown is predictable and its resources are released

  @captain
  Scenario: SIGTERM stops the server with a clean exit
    Given the finger server is started on port 0
    When the process receives the signal SIGTERM
    Then the process exits within one second with code 0
